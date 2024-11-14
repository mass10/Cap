use cap_media::feeds::AudioInputSamples;
use cpal::{SampleFormat, StreamInstant};
use keyed_priority_queue::KeyedPriorityQueue;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::time::Duration;

pub const MAX_AMPLITUDE_F32: f64 = (u16::MAX / 2) as f64; // i16 max value
pub const ZERO_AMPLITUDE: u16 = 0;
pub const TERMINAL_WIDTH: f64 = 0.8;
pub const MIN_DB: f64 = -96.0;

// https://github.com/cgbur/meter/blob/master/src/time_window.rs
pub(crate) struct VolumeMeter {
    pub(crate) keep_secs: f32,
    keep_duration: Duration, // secs
    maxes: KeyedPriorityQueue<StreamInstant, MinNonNan>,
    times: VecDeque<StreamInstant>,
}

impl VolumeMeter {
    pub fn new(keep_secs: f32) -> Self {
        Self {
            keep_duration: Duration::from_secs_f32(keep_secs),
            keep_secs,
            maxes: KeyedPriorityQueue::new(),
            times: Default::default(),
        }
    }

    pub fn push(&mut self, time: StreamInstant, value: f64) {
        let value = MinNonNan(-value);
        self.maxes.push(time, value);
        self.times.push_back(time);

        loop {
            if let Some(time) = self
                .times
                .back()
                .unwrap()
                .duration_since(self.times.front().unwrap())
            {
                if time > self.keep_duration {
                    self.maxes.remove(self.times.front().unwrap());
                    self.times.pop_front();
                } else {
                    break;
                }
            } else {
                break;
            }

            if self.times.len() <= 1 {
                break;
            }
        }
    }

    pub fn max(&self) -> f64 {
        -self.maxes.peek().map(|(_, db)| db.0).unwrap_or(-MIN_DB)
    }
}

#[derive(PartialEq)]
struct MinNonNan(f64);

impl Eq for MinNonNan {}

impl PartialOrd for MinNonNan {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for MinNonNan {
    fn cmp(&self, other: &MinNonNan) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

use cpal::Sample;

pub fn db_fs(data: impl Iterator<Item = f64>) -> f64 {
    let max = data
        .map(|f| f.to_sample::<i16>().unsigned_abs())
        .max()
        .unwrap_or(ZERO_AMPLITUDE);

    (20.0 * (max as f64 / MAX_AMPLITUDE_F32).log10()).clamp(MIN_DB, 0.0)
}

pub fn samples_to_f64(samples: &AudioInputSamples) -> impl Iterator<Item = f64> + use<'_> {
    samples
        .data
        .chunks(samples.format.sample_size())
        .map(|data| match samples.format {
            SampleFormat::I8 => i8::from_ne_bytes([data[0]]) as f64 / i8::MAX as f64,
            SampleFormat::U8 => u8::from_ne_bytes([data[0]]) as f64 / u8::MAX as f64,
            SampleFormat::I16 => i16::from_ne_bytes([data[0], data[1]]) as f64 / i16::MAX as f64,
            SampleFormat::U16 => u16::from_ne_bytes([data[0], data[1]]) as f64 / u16::MAX as f64,
            SampleFormat::I32 => {
                i32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as f64 / i32::MAX as f64
            }
            SampleFormat::U32 => {
                u32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as f64 / u32::MAX as f64
            }
            SampleFormat::U64 => {
                u64::from_ne_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]) as f64
                    / u64::MAX as f64
            }
            SampleFormat::I64 => {
                i64::from_ne_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]) as f64
                    / i64::MAX as f64
            }
            SampleFormat::F32 => f32::from_ne_bytes([data[0], data[1], data[2], data[3]]) as f64,
            SampleFormat::F64 => f64::from_ne_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]),
            _ => todo!(),
        })
}
