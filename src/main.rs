use coreaudio::audio_unit::AudioUnit;
use coreaudio::audio_unit::render_callback;
use coreaudio::audio_unit::{SampleFormat, Scope};
use coreaudio::sys::{
    AudioDeviceID,
};
mod audio_dev_helpers;
mod audio_unit_helpers;
mod utils;

use crate::audio_dev_helpers::{
  find_matching_dev,
  identify_me_to_dad,
  set_physical_device_format,
};
use crate::audio_unit_helpers::{
    setup_audio_unit,
    Callback,
    CBArgs,
};
use crate::utils::type_of;

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

fn mom_mode_from_cli_args(cli_args: Vec<String>) -> &'static str {
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

fn stream_audio(mom_mode: &str) -> Result<(), Box<dyn Error>> {
    let sample_rate: f64 = 48_000.0;
    let sample_format: SampleFormat = SampleFormat::F32;
    let n_channels: u32 = 2;

    let devid: AudioDeviceID = setup_connection_to_dad(
        mom_mode,
        "Meta Audio Debug",
        sample_rate,
        sample_format,
        n_channels,
    )?;

    let audio_unit_cb: Box<Callback> = get_audio_unit_callback();
    let (mut audio_unit, mut rate_listener, mut alive_listener) =
        setup_audio_unit(devid, sample_rate, sample_format, n_channels, audio_unit_cb)?;

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

fn write_to_file(mom_mode: &str) -> Result<(), Box<dyn Error>> {
    let fname: String = format!("{}.pcm", mom_mode);
    println!("writing to file {}", fname);
    let mut file: File = File::create(fname)?;
    unsafe {
        let ptr = audio_buf.as_ptr();
        let bytes: &[u8] = std::slice::from_raw_parts(
            audio_buf.as_ptr() as *const u8,
            audio_buf.len() * std::mem::size_of::<f32>(),
        );
        file.write_all(bytes)?;
    }
    println!("done.");
    Ok(())
}

fn setup_connection_to_dad(
    mom_mode: &str,
    dad_name_match_str: &str,
    sample_rate: f64,
    sample_format: SampleFormat,
    n_channels: u32,
) -> Result<AudioDeviceID, Box<dyn Error>> {
    let (devid, name) = find_matching_dev("Meta Audio Debug", Scope::Input)?;
    // print_supported_formats(devid)?;
    identify_me_to_dad(devid, mom_mode)?;
    set_physical_device_format(devid, sample_rate, sample_format, n_channels)?;
    Ok(devid)
}


static expected_step: f32 = 0.05764f32;
static mut last_val: f32 = 0.000f32;

fn get_audio_unit_callback() -> Box<Callback> {
    Box::new(move |args| -> Result<(), ()> {
        let num_frames: usize = args.num_frames;
        let data: render_callback::data::Interleaved<f32> = args.data;
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
        Ok(())
    })
}

fn write_data(
    num_frames: usize,
    data: render_callback::data::Interleaved<f32>,
) -> Result<usize, Box<dyn Error>> {
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
        audio_buf.extend_from_slice(&data.buffer[0..num_frames * data.channels]);
    }
    Ok(num_frames)
}
