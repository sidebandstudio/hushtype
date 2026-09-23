/// Conservative final gate for 16 kHz mono PCM. The recorder's VAD remains
/// responsible for speech detection; this only rejects tiny or near-zero input.
pub fn is_near_silent(samples: &[f32]) -> bool {
    samples.len() < 1600
        || !samples.chunks(320).any(|frame| {
            let energy: f64 = frame.iter().map(|&s| f64::from(s).powi(2)).sum();
            energy.is_finite() && energy / frame.len() as f64 > 0.0005_f64.powi(2)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_silent_and_near_silent_inputs_are_skipped() {
        assert!(is_near_silent(&[]));
        assert!(is_near_silent(&vec![0.1; 800]));
        assert!(is_near_silent(&vec![0.0; 16000]));
        assert!(is_near_silent(&vec![0.0001; 16000]));
        assert!(is_near_silent(&vec![f32::NAN; 16000]));
    }
    #[test]
    fn soft_audio_is_not_discarded() {
        let mut samples = vec![0.0; 16000];
        samples[8000..9000].fill(0.003);
        assert!(!is_near_silent(&samples));
    }
}
