use anyhow::Result;
use ndarray::{Array1, Array2, Array3};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::path::Path;

use super::{VadFrame, VoiceActivityDetector};
use crate::audio_toolkit::constants;

const SILERO_FRAME_MS: u32 = 30;
const SILERO_FRAME_SAMPLES: usize =
    (constants::WHISPER_SAMPLE_RATE * SILERO_FRAME_MS / 1000) as usize;

/// Silero VAD v4 carries a two-layer LSTM (hidden and cell state, width 64)
/// from one frame to the next.
const STATE_SHAPE: (usize, usize, usize) = (2, 1, 64);

/// One small single-threaded session: frames arrive every 30 ms, so extra
/// threads would only add scheduling overhead.
fn open_session(model_path: &Path) -> ort::Result<Session> {
    let mut builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort::Error::<()>::from)?
        .with_intra_threads(1)
        .map_err(ort::Error::<()>::from)?
        .with_inter_threads(1)
        .map_err(ort::Error::<()>::from)?;
    builder.commit_from_file(model_path)
}

pub struct SileroVad {
    session: Session,
    hidden: Array3<f32>,
    cell: Array3<f32>,
    threshold: f32,
}

impl SileroVad {
    pub fn new<P: AsRef<Path>>(model_path: P, threshold: f32) -> Result<Self> {
        if !(0.0..=1.0).contains(&threshold) {
            anyhow::bail!("threshold must be between 0.0 and 1.0");
        }

        let session = open_session(model_path.as_ref())
            .map_err(|e| anyhow::anyhow!("Failed to create VAD: {e}"))?;

        Ok(Self {
            session,
            hidden: Array3::zeros(STATE_SHAPE),
            cell: Array3::zeros(STATE_SHAPE),
            threshold,
        })
    }

    /// Run one frame through the model and carry its recurrent state forward.
    fn speech_probability(&mut self, frame: &[f32]) -> Result<f32> {
        let input = Value::from_array(Array2::from_shape_vec((1, frame.len()), frame.to_vec())?)?;
        let sample_rate = Value::from_array(Array1::from_vec(vec![i64::from(
            constants::WHISPER_SAMPLE_RATE,
        )]))?;
        let hidden = Value::from_array(self.hidden.clone())?;
        let cell = Value::from_array(self.cell.clone())?;

        let outputs = self.session.run(ort::inputs![
            "input" => input,
            "sr" => sample_rate,
            "h" => hidden,
            "c" => cell
        ])?;

        let (_, hidden) = outputs["hn"].try_extract_tensor::<f32>()?;
        self.hidden = Array3::from_shape_vec(STATE_SHAPE, hidden.to_vec())?;
        let (_, cell) = outputs["cn"].try_extract_tensor::<f32>()?;
        self.cell = Array3::from_shape_vec(STATE_SHAPE, cell.to_vec())?;
        let (_, probability) = outputs["output"].try_extract_tensor::<f32>()?;
        probability
            .first()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Silero VAD returned no probability"))
    }
}

impl VoiceActivityDetector for SileroVad {
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>> {
        if frame.len() != SILERO_FRAME_SAMPLES {
            anyhow::bail!(
                "expected {SILERO_FRAME_SAMPLES} samples, got {}",
                frame.len()
            );
        }

        let probability = self
            .speech_probability(frame)
            .map_err(|e| anyhow::anyhow!("Silero VAD error: {e}"))?;

        if probability > self.threshold {
            Ok(VadFrame::Speech(frame))
        } else {
            Ok(VadFrame::Noise)
        }
    }

    fn frame_samples(&self) -> usize {
        SILERO_FRAME_SAMPLES
    }

    fn reset(&mut self) {
        // Clear the Silero LSTM hidden/cell state so a new session doesn't
        // inherit recurrent context from the previous recording.
        self.hidden.fill(0.0);
        self.cell.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/models/silero_vad_v4.onnx")
    }

    #[test]
    fn silence_is_noise_and_reset_restores_initial_state() {
        let mut vad = SileroVad::new(model_path(), 0.3).unwrap();
        let silence = vec![0.0; SILERO_FRAME_SAMPLES];
        let tone: Vec<f32> = (0..SILERO_FRAME_SAMPLES)
            .map(|i| (i as f32 * 0.07).sin() * 0.5)
            .collect();

        assert!(!vad.is_voice(&silence).unwrap());
        let first = vad.speech_probability(&tone).unwrap();
        vad.speech_probability(&tone).unwrap();
        vad.reset();
        vad.is_voice(&silence).unwrap();
        assert_eq!(vad.speech_probability(&tone).unwrap(), first);
    }

    #[test]
    fn rejects_wrong_frame_size() {
        let mut vad = SileroVad::new(model_path(), 0.3).unwrap();
        assert!(vad.push_frame(&[0.0; 512]).is_err());
    }
}
