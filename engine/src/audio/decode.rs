//! OGG Vorbis decoding (via symphonia) into interleaved stereo i16 at the mixer rate.

use std::io::Cursor;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::mixer::MusicSource;
use super::SAMPLE_RATE;

/// Incrementally decodes an embedded OGG Vorbis file.
pub struct VorbisStream {
    bytes: &'static [u8],
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    channels: usize,
    sample_buffer: Option<SampleBuffer<i16>>,
    /// Decoded-but-unread interleaved samples, and how many of them have been consumed.
    pending: Vec<i16>,
    pending_pos: usize,
    finished: bool,
}

impl VorbisStream {
    pub fn new(bytes: &'static [u8]) -> Result<Self, String> {
        let stream = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("ogg");
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("unsupported audio container: {e}"))?;
        let format = probed.format;
        let track = format
            .default_track()
            .ok_or_else(|| "audio file has no tracks".to_string())?;
        let params = &track.codec_params;
        let sample_rate = params
            .sample_rate
            .ok_or_else(|| "audio track has no sample rate".to_string())?;
        if sample_rate != SAMPLE_RATE {
            return Err(format!(
                "audio track is {sample_rate} Hz but the mixer runs at {SAMPLE_RATE} Hz"
            ));
        }
        let channels = params
            .channels
            .map(|c| c.count())
            .ok_or_else(|| "audio track has no channel layout".to_string())?;
        if !(1..=2).contains(&channels) {
            return Err(format!(
                "audio track has {channels} channels; only mono/stereo are supported"
            ));
        }
        let decoder = symphonia::default::get_codecs()
            .make(params, &DecoderOptions::default())
            .map_err(|e| format!("unsupported audio codec: {e}"))?;
        Ok(Self {
            bytes,
            track_id: track.id,
            format,
            decoder,
            channels,
            sample_buffer: None,
            pending: Vec::new(),
            pending_pos: 0,
            finished: false,
        })
    }

    /// Decodes the next packet into `pending`. Returns false at end of stream.
    fn fill(&mut self) -> bool {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(_) => return false,
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(_) => return false,
            };
            if decoded.frames() == 0 {
                continue;
            }
            let buffer = self.sample_buffer.get_or_insert_with(|| {
                SampleBuffer::new(decoded.capacity() as u64, *decoded.spec())
            });
            buffer.copy_interleaved_ref(decoded);
            self.pending.clear();
            self.pending_pos = 0;
            if self.channels == 1 {
                self.pending
                    .extend(buffer.samples().iter().flat_map(|&s| [s, s]));
            } else {
                self.pending.extend_from_slice(buffer.samples());
            }
            return true;
        }
    }

    /// Decodes the whole file into interleaved stereo samples.
    pub fn decode_all(mut self) -> Vec<i16> {
        let mut out = Vec::new();
        while self.fill() {
            out.extend_from_slice(&self.pending);
        }
        out
    }
}

impl MusicSource for VorbisStream {
    fn read(&mut self, out: &mut [i16]) -> usize {
        let mut written = 0;
        while written < out.len() {
            if self.pending_pos >= self.pending.len() {
                if self.finished || !self.fill() {
                    self.finished = true;
                    break;
                }
            }
            let available = &self.pending[self.pending_pos..];
            let n = available.len().min(out.len() - written);
            out[written..written + n].copy_from_slice(&available[..n]);
            written += n;
            self.pending_pos += n;
        }
        written
    }

    fn reset(&mut self) {
        // Re-opening is simpler and more reliable than seeking an OGG stream back to 0.
        if let Ok(fresh) = VorbisStream::new(self.bytes) {
            *self = fresh;
        }
    }
}

pub fn decode_all(bytes: &'static [u8]) -> Result<Vec<i16>, String> {
    Ok(VorbisStream::new(bytes)?.decode_all())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOVE: &[u8] = include_bytes!("test/stereo.ogg");
    const MONO: &[u8] = include_bytes!("test/mono.ogg");

    #[test]
    fn decodes_stereo_sfx() {
        let pcm = decode_all(MOVE).unwrap();
        assert!(pcm.len() > 1000 && pcm.len() % 2 == 0);
        assert!(pcm.iter().any(|&s| s != 0));
    }

    #[test]
    fn streaming_matches_full_decode_and_reset_restarts() {
        let all = decode_all(MOVE).unwrap();
        let mut stream = VorbisStream::new(MOVE).unwrap();
        let mut chunked = vec![0i16; all.len() + 64];
        let mut total = 0;
        loop {
            let end = (total + 100).min(chunked.len());
            let n = stream.read(&mut chunked[total..end]);
            if n == 0 {
                break;
            }
            total += n;
        }
        assert_eq!(&chunked[..total], &all[..]);
        stream.reset();
        let mut again = vec![0i16; 50];
        assert_eq!(stream.read(&mut again), 50);
        assert_eq!(&again[..], &all[..50]);
    }

    #[test]
    fn mono_is_upmixed_to_stereo() {
        let pcm = decode_all(MONO).unwrap();
        assert!(pcm.chunks(2).all(|c| c[0] == c[1]));
    }

    #[test]
    fn garbage_is_an_error() {
        assert!(VorbisStream::new(b"not an ogg file at all").is_err());
    }
}
