use coreaudio::audio_unit::AudioUnit;
use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers::{
    audio_unit_from_device_id, find_matching_physical_format, get_audio_device_ids,
    get_audio_device_supports_scope, get_device_name, get_supported_physical_stream_formats,
    set_device_physical_stream_format, AliveListener, RateListener,
};
use coreaudio::audio_unit::render_callback::{self, data};
use coreaudio::audio_unit::{Element, SampleFormat, Scope, StreamFormat};
use coreaudio::sys::{
    kAudioUnitProperty_StreamFormat,
    AudioDeviceID,
};
mod au_helpers;
use crate::au_helpers::*;

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;


const CAP_MODE: &str = "capture";
const PLAY_MODE: &str = "playout";
const SINE440_MODE: &str = "sine440";
const SINE880_MODE: &str = "sine880";

const BUFSIZE: usize = 1000 * 1000 * 10 * std::mem::size_of::<f32>();
static mut audio_buf: Vec<f32> = Vec::new();

fn main() -> Result<(), Box<dyn Error>> {

    let mut mom_mode: &str = mom_mode_from_cli_args(env::args().collect());

    // setup exchange buffer
    unsafe {
        audio_buf.reserve(BUFSIZE);
    }

    stream_audio(mom_mode);
    write_to_file(mom_mode);

    Ok(())
}

fn write_to_file(mom_mode: &str) -> Result<(), Box<dyn Error>> {
  println!("writing to file {}", mom_mode);
  let mut file: File = File::create(mom_mode)?;
  unsafe {
      let ptr = audio_buf.as_ptr();
      let bytes: &[u8] = std::slice::from_raw_parts(
          audio_buf.as_ptr() as *const u8,
          audio_buf.len() * std::mem::size_of::<f32>(),
      );
      file.write_all(bytes)?;
  }
  println!("done.");
}

fn mom_mode_from_cli_args(cli_args: &Vec<String>) -> &'static str {
    let mut mom_mode: &str = CAP_MODE;
    if cli_args.len() > 1 {
        if cli_args[1].starts_with("cap") {
            mom_mode = CAP_MODE;
        } else if cli_args[1].starts_with("play") {
            mom_mode = PLAY_MODE;
        } else if cli_args[1].contains("440") {
            mom_mode = SINE440_MODE;
        } else if cli_args[1].contains("880") {
            mom_mode = SINE880_MODE;
        }
    }
    return mom_mode;
}

fn find_matching_dev(
    name_substr: &str,
    scope: Scope,
) -> Result<(AudioDeviceID, String), &'static str> {
    let dev_ids = get_audio_device_ids().unwrap();
    let mut matching: Vec<String> = Vec::new();
    for devid in dev_ids {
        let name = get_device_name(devid).unwrap();
        if name.contains(name_substr) {
            let supports_scope = get_audio_device_supports_scope(devid, scope);
            let it_is: bool = match supports_scope {
                Ok(is) => is,
                Err(e) => false,
            };
            if it_is {
                return Ok((devid, name));
            }
        }
    }
    return Err("DAD device not found");
}

fn print_supported_formats(devid: AudioDeviceID) -> Result<(), Box<dyn Error>> {
    // print supported formats, disabled for now since it often crashes.
    println!("All supported formats");
    let formats = get_supported_physical_stream_formats(devid)?;
    for fmt in formats {
        println!("supported type={}\n{:?}", type_of(&fmt), &fmt);
    }

    Ok(())
}

fn set_physical_device_format(
    devid: AudioDeviceID,
    sample_rate: f64,
    sample_format: SampleFormat,
    n_channels: u32,
) -> Result<(), Box<dyn Error>> {
    // set physical device format
    let hw_stream_format = StreamFormat {
        sample_rate: sample_rate,
        sample_format: sample_format,
        flags: LinearPcmFlags::empty(),
        channels: n_channels,
    };
    println!("physical device stream format = {:#?}", hw_stream_format);

    let hw_asbd = find_matching_physical_format(devid, hw_stream_format)
        .ok_or(coreaudio::Error::UnsupportedStreamFormat)?;
    println!("hw_asbd = {:#?}", hw_asbd);
    set_device_physical_stream_format(devid, hw_asbd)?;
    Ok(())
}

fn set_audiounit_format(
    audio_unit: &mut AudioUnit,
    sample_rate: f64,
    sample_format: SampleFormat,
    n_channels: u32,
) -> Result<(), Box<dyn Error>> {

  let format_flag = LinearPcmFlags::IS_FLOAT | LinearPcmFlags::IS_PACKED;
  let au_stream_format = StreamFormat {
      sample_rate: sample_rate,
      sample_format: sample_format,
      flags: format_flag,
      channels: n_channels,
  };
  println!(
      "AudioUnit stream format type={}\n{:#?}",
      type_of(&au_stream_format),
      &au_stream_format
  );
  let au_asbd = au_stream_format.to_asbd();

  let fmtid = kAudioUnitProperty_StreamFormat;
  audio_unit.set_property(fmtid, Scope::Output, Element::Input, Some(&au_asbd))?;

  // Check the AudioUnit's format is correct
  assert!(au_stream_format.sample_format == sample_format);
  Ok(())
}

fn setup_au_listeners(devid: AudioDeviceID) -> Result<(RateListener, AliveListener), Box<dyn Error>> {
  let mut rate_listener = RateListener::new(devid, None);
  rate_listener.register()?;
  let mut alive_listener = AliveListener::new(devid);
  alive_listener.register()?;
  Ok((rate_listener, alive_listener))
}

fn stream_audio(mom_mode: &str) -> Result<(), Box<dyn Error>> {
	let sample_rate: f64 = 48_000.0;
	let sample_format: SampleFormat = SampleFormat::F32;
	let n_channels: u32 = 2;
    let (devid, name) = find_matching_dev("Meta Audio Debug", Scope::Input).unwrap();
    let mut audio_unit = audio_unit_from_device_id(devid, true).unwrap();

    identify_me_to_dad(devid, mom_mode)?;

    // print_supported_formats(devid)?;

    set_physical_device_format(devid, sample_rate, sample_format, n_channels)?;
    set_audiounit_format(&mut audio_unit, sample_rate, sample_format, n_channels)?;

    let (mut rate_listener, mut alive_listener) = setup_au_listeners(devid)?;

    type Args = render_callback::Args<data::Interleaved<f32>>;
    audio_unit.set_input_callback(move |args| -> Result<(), ()> {
        let Args {
            num_frames, data, ..
        } = args;
        match write_data(num_frames, data) {
            Err(e) => println!("Error: {}", e),
            Ok(_) => {}
        };
        Ok(())
    })?;

    println!("streaming audio...");
    audio_unit.start()?;

    for _ in 0..2 {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        // print all sample change events
        println!("rate events: {:?}", rate_listener.copy_values());
        println!("alive state: {}", alive_listener.is_alive());
    }
    Ok(())
}

static expected_step: f32 = 0.05764f32;
static mut last_val: f32 = 0.000f32;

fn write_data(num_frames: usize, data: data::Interleaved<f32>) -> Result<usize, Box<dyn Error>> {
    unsafe {
        let next_val: f32 = data.buffer[0];
        let diff: f32 = next_val - last_val;
        if (diff.abs() > expected_step * 1.01) {
            println!(
                "[1;33mbig jump: last={:.4} next={:.4}[0m",
                last_val, next_val
            );
        }
        last_val = data.buffer[data.buffer.len() - 1];
        audio_buf.extend_from_slice(&data.buffer[0..num_frames * 2]);
    }
    Ok(num_frames)
}

fn type_of<T>(_: &T) -> &str {
    std::any::type_name::<T>()
}
