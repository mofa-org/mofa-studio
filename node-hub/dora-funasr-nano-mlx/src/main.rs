//! Dora ASR Node using MLX-based FunASR-Nano (SenseVoice + Qwen3)
//!
//! This node provides multi-language speech-to-text transcription optimized for Apple Silicon
//! using the SenseVoice encoder and Qwen3-0.6B LLM via MLX (Metal acceleration).
//!
//! Features:
//! - Automatic chunking for audio longer than 30 seconds
//! - Overlapping chunks for better accuracy at boundaries

use dora_node_api::{
    arrow::array::{Array, ArrayRef, AsArray, Float32Array, Float64Array, Int16Array, StringArray},
    dora_core::config::{DataId, NodeId},
    DoraNode, Event, Parameter,
};
use eyre::{Context, Result};
use std::collections::BTreeMap;
use std::time::Instant;

mod config;
mod engine;

use config::Config;
use engine::SenseVoiceEngine;

/// Maximum audio duration per chunk (model limit is 30s)
const CHUNK_DURATION_SECS: f64 = 28.0;
/// Overlap between chunks for better boundary handling
const OVERLAP_DURATION_SECS: f64 = 1.0;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting dora-funasr-nano-mlx (SenseVoice+Qwen) node...");

    // Load configuration
    let config = Config::from_env();
    log::info!("Configuration loaded: {:?}", config);

    // Create Dora node (supports --name <id> for dynamic mode)
    let args: Vec<String> = std::env::args().collect();
    let node_id = if args.len() > 2 && args[1] == "--name" {
        Some(args[2].clone())
    } else {
        None
    };

    let (mut node, events) = if let Some(id) = node_id {
        log::info!("Initializing as dynamic node: {}", id);
        DoraNode::init_from_node_id(NodeId::from(id))?
    } else {
        DoraNode::init_from_env()?
    };

    // Initialize ASR engine
    let mut engine = if config.warmup {
        log::info!("Pre-initializing SenseVoice engine...");
        let start = Instant::now();
        match SenseVoiceEngine::new(&config.model_dir) {
            Ok(e) => {
                log::info!("Engine initialized in {:.2}s", start.elapsed().as_secs_f32());
                Some(e)
            }
            Err(e) => {
                log::error!("Failed to initialize engine: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Statistics tracking
    let mut total_segments: u64 = 0;
    let mut total_audio_duration: f64 = 0.0;
    let mut total_processing_time: f64 = 0.0;

    // Event loop
    let events = futures::executor::block_on_stream(events);

    for event in events {
        match event {
            Event::Input { id, data, metadata } => {
                let input_id = id.as_str();

                match input_id {
                    "audio" => {
                        // Lazily initialize engine if not warmed up
                        if engine.is_none() {
                            log::info!("Lazily initializing SenseVoice engine...");
                            let start = Instant::now();
                            match SenseVoiceEngine::new(&config.model_dir) {
                                Ok(e) => {
                                    log::info!(
                                        "Engine initialized in {:.2}s",
                                        start.elapsed().as_secs_f32()
                                    );
                                    engine = Some(e);
                                }
                                Err(e) => {
                                    log::error!("Failed to initialize engine: {}", e);
                                    send_log(
                                        &mut node,
                                        "ERROR",
                                        &format!("Engine initialization failed: {}", e),
                                    )?;
                                    continue;
                                }
                            }
                        }

                        let engine = match engine.as_mut() {
                            Some(e) => e,
                            None => {
                                log::error!("Engine unexpectedly missing after attempted initialization");
                                send_log(&mut node, "ERROR", "Engine not available")?;
                                continue;
                            }
                        };

                        // Extract metadata
                        let question_id = metadata
                            .parameters
                            .get("question_id")
                            .and_then(|p| match p {
                                Parameter::String(s) => s.parse::<u64>().ok(),
                                Parameter::Integer(i) => Some(*i as u64),
                                _ => None,
                            });
                        let segment = metadata.parameters.get("segment").and_then(|p| match p {
                            Parameter::String(s) => s.parse::<u64>().ok(),
                            Parameter::Integer(i) => Some(*i as u64),
                            _ => None,
                        });
                        let sample_rate =
                            metadata
                                .parameters
                                .get("sample_rate")
                                .and_then(|p| match p {
                                    Parameter::String(s) => s.parse::<u32>().ok(),
                                    Parameter::Integer(i) => Some(*i as u32),
                                    _ => None,
                                })
                                .unwrap_or(16000);

                        // Convert Arrow data to f32 samples
                        let samples = match extract_audio_samples(&data) {
                            Ok(s) => s,
                            Err(e) => {
                                log::error!("Failed to extract audio samples: {}", e);
                                send_log(
                                    &mut node,
                                    "ERROR",
                                    &format!("Audio extraction failed: {}", e),
                                )?;
                                continue;
                            }
                        };

                        // Calculate audio duration
                        let audio_duration = samples.len() as f64 / sample_rate as f64;

                        // Validate audio duration
                        if audio_duration < config.min_audio_duration {
                            log::warn!(
                                "Audio too short: {:.2}s < {:.2}s minimum",
                                audio_duration,
                                config.min_audio_duration
                            );
                            send_log(
                                &mut node,
                                "WARN",
                                &format!("Audio too short: {:.2}s", audio_duration),
                            )?;
                            continue;
                        }

                        // Process audio (with chunking for long audio)
                        process_audio_with_chunking(
                            &mut node,
                            engine,
                            &samples,
                            sample_rate,
                            question_id,
                            segment,
                            &mut total_segments,
                            &mut total_audio_duration,
                            &mut total_processing_time,
                        )?;
                    }
                    "control" => {
                        let text_array = data.as_string::<i32>();
                        let command = text_array
                            .iter()
                            .filter_map(|s| s)
                            .collect::<Vec<_>>()
                            .join("");

                        handle_control_command(
                            &mut node,
                            &command,
                            total_segments,
                            total_audio_duration,
                            total_processing_time,
                        )?;
                    }
                    _ => {
                        log::debug!("Ignoring unknown input: {}", input_id);
                    }
                }
            }
            Event::Stop(_) => {
                log::info!("Received stop signal, shutting down...");
                break;
            }
            _ => {}
        }
    }

    log::info!("dora-funasr-nano-mlx node stopped");
    Ok(())
}

/// Split audio into overlapping chunks for long audio processing
fn split_audio_into_chunks(samples: &[f32], sample_rate: u32) -> Vec<Vec<f32>> {
    let chunk_samples = (CHUNK_DURATION_SECS * sample_rate as f64) as usize;
    let overlap_samples = (OVERLAP_DURATION_SECS * sample_rate as f64) as usize;
    let step_samples = chunk_samples - overlap_samples;
    let min_chunk_samples = sample_rate as usize; // Minimum 1 second

    let mut chunks = Vec::new();
    let mut i = 0;

    while i < samples.len() {
        let end = (i + chunk_samples).min(samples.len());
        let chunk = samples[i..end].to_vec();

        // Skip very short final chunks
        if chunk.len() >= min_chunk_samples {
            chunks.push(chunk);
        }

        i += step_samples;

        // If the remaining audio is less than minimum, include it in the last chunk
        if i < samples.len() && samples.len() - i < min_chunk_samples {
            break;
        }
    }

    chunks
}

/// Merge transcription chunks into a single text
fn merge_transcriptions(transcriptions: &[String]) -> String {
    if transcriptions.is_empty() {
        return String::new();
    }
    if transcriptions.len() == 1 {
        return transcriptions[0].clone();
    }

    // Simple merge with space separator
    // Could be improved with overlap detection for better accuracy
    transcriptions
        .iter()
        .filter(|t| !t.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("")
}

/// Process audio with automatic chunking for long audio
fn process_audio_with_chunking(
    node: &mut DoraNode,
    engine: &mut SenseVoiceEngine,
    samples: &[f32],
    sample_rate: u32,
    question_id: Option<u64>,
    segment: Option<u64>,
    total_segments: &mut u64,
    total_audio_duration: &mut f64,
    total_processing_time: &mut f64,
) -> Result<()> {
    let audio_duration = samples.len() as f64 / sample_rate as f64;

    log::info!(
        "Processing audio: {:.2}s, question_id={:?}, segment={:?}",
        audio_duration,
        question_id,
        segment
    );

    let start = Instant::now();

    // Check if we need to chunk the audio
    let (transcription, language) = if audio_duration > CHUNK_DURATION_SECS {
        let chunks = split_audio_into_chunks(samples, sample_rate);
        log::info!(
            "Audio too long ({:.2}s), splitting into {} chunks",
            audio_duration,
            chunks.len()
        );

        let mut transcriptions = Vec::new();
        let mut detected_language = String::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_duration = chunk.len() as f64 / sample_rate as f64;
            log::info!(
                "Processing chunk {}/{} ({:.2}s)...",
                i + 1,
                chunks.len(),
                chunk_duration
            );

            match engine.transcribe(chunk, sample_rate) {
                Ok((text, lang)) => {
                    log::debug!("Chunk {} transcription: '{}'", i + 1, text);
                    transcriptions.push(text);
                    if detected_language.is_empty() {
                        detected_language = lang;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to transcribe chunk {}: {}", i + 1, e);
                }
            }
        }

        let merged = merge_transcriptions(&transcriptions);
        if detected_language.is_empty() {
            detected_language = "unknown".to_string();
        }

        (merged, detected_language)
    } else {
        // Short audio - process directly
        engine.transcribe(samples, sample_rate)?
    };

    let processing_time = start.elapsed().as_secs_f64();

    // Update statistics
    *total_segments += 1;
    *total_audio_duration += audio_duration;
    *total_processing_time += processing_time;

    let rtf = processing_time / audio_duration;
    log::info!(
        "Transcription complete: '{}' (lang={}, {:.2}s, RTF={:.3})",
        transcription,
        language,
        processing_time,
        rtf
    );

    // Build output metadata
    let mut metadata: BTreeMap<String, Parameter> = BTreeMap::new();
    if let Some(qid) = question_id {
        metadata.insert(
            "question_id".to_string(),
            Parameter::String(qid.to_string()),
        );
    }
    if let Some(seg) = segment {
        metadata.insert("segment".to_string(), Parameter::String(seg.to_string()));
    }
    metadata.insert(
        "session_status".to_string(),
        Parameter::String("ended".to_string()),
    );

    // Send outputs
    node.send_output(
        DataId::from("transcription".to_string()),
        metadata,
        StringArray::from(vec![transcription.as_str()]),
    )
    .context("Failed to send transcription")?;

    node.send_output(
        DataId::from("language_detected".to_string()),
        BTreeMap::new(),
        StringArray::from(vec![language.as_str()]),
    )
    .context("Failed to send language_detected")?;

    node.send_output(
        DataId::from("processing_time".to_string()),
        BTreeMap::new(),
        StringArray::from(vec![processing_time.to_string().as_str()]),
    )
    .context("Failed to send processing_time")?;

    send_log(
        node,
        "INFO",
        &format!(
            "Transcribed {:.2}s audio in {:.2}s (RTF={:.3}, lang={})",
            audio_duration, processing_time, rtf, language,
        ),
    )?;

    Ok(())
}

fn handle_control_command(
    node: &mut DoraNode,
    command: &str,
    total_segments: u64,
    total_audio_duration: f64,
    total_processing_time: f64,
) -> Result<()> {
    let command = command.trim().to_lowercase();
    log::info!("Received control command: {}", command);

    match command.as_str() {
        "stats" => {
            let avg_rtf = if total_audio_duration > 0.0 {
                total_processing_time / total_audio_duration
            } else {
                0.0
            };

            let stats = serde_json::json!({
                "total_segments": total_segments,
                "total_audio_duration": total_audio_duration,
                "total_processing_time": total_processing_time,
                "average_rtf": avg_rtf,
                "engine": "sensevoice-nano-mlx"
            });

            send_log(node, "INFO", &format!("Stats: {}", stats))?;
        }
        "cleanup" | "reset" => {
            send_log(node, "INFO", "Engine reset requested")?;
        }
        _ => {
            log::warn!("Unknown control command: {}", command);
        }
    }

    Ok(())
}

fn extract_audio_samples(data: &ArrayRef) -> Result<Vec<f32>> {
    // Try Float32Array first
    if let Some(float_array) = data.as_any().downcast_ref::<Float32Array>() {
        return Ok(float_array.values().to_vec());
    }

    // Try Float64Array and convert
    if let Some(float64_array) = data.as_any().downcast_ref::<Float64Array>() {
        return Ok(float64_array.values().iter().map(|&x| x as f32).collect());
    }

    // Try Int16Array and convert (PCM format)
    if let Some(int16_array) = data.as_any().downcast_ref::<Int16Array>() {
        return Ok(int16_array
            .values()
            .iter()
            .map(|&x| x as f32 / 32768.0)
            .collect());
    }

    Err(eyre::eyre!(
        "Unsupported audio format. Expected Float32Array, Float64Array, or Int16Array"
    ))
}

fn send_log(node: &mut DoraNode, level: &str, message: &str) -> Result<()> {
    let log_entry = serde_json::json!({
        "level": level,
        "node": "dora-funasr-nano-mlx",
        "message": message,
        "timestamp": chrono::Utc::now().timestamp()
    });

    node.send_output(
        DataId::from("log".to_string()),
        BTreeMap::new(),
        StringArray::from(vec![log_entry.to_string().as_str()]),
    )
    .context("Failed to send log")?;

    Ok(())
}
