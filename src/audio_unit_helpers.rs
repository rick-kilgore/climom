use coreaudio::audio_unit::AudioUnit;
use coreaudio::audio_unit::{Element, SampleFormat, Scope, StreamFormat};
use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers::{
    audio_unit_from_device_id, AliveListener, RateListener,
};
use coreaudio::audio_unit::render_callback;
use coreaudio::sys::{
    kAudioUnitProperty_StreamFormat,
    AudioDeviceID,
};

use std::error::Error;

use crate::utils::type_of;

pub type CBArgs = render_callback::Args<render_callback::data::Interleaved<f32>>;
pub type Callback = dyn FnMut(CBArgs) -> Result<(), ()> + 'static;

pub fn setup_audio_unit(
    devid: AudioDeviceID,
    sample_rate: f64,
    sample_format: SampleFormat,
    n_channels: u32,
    mut audio_unit_cb: Box<Callback>,
) -> Result<(AudioUnit, RateListener, AliveListener), Box<dyn Error>> {
    let mut audio_unit = audio_unit_from_device_id(devid, true)?;
    let (mut rate_listener, mut alive_listener) = setup_au_listeners(devid)?;
    set_audiounit_format(&mut audio_unit, sample_rate, sample_format, n_channels)?;

    audio_unit.set_input_callback(audio_unit_cb)?;

    Ok((audio_unit, rate_listener, alive_listener))
}

pub fn set_audiounit_format(
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

pub fn setup_au_listeners(
    devid: AudioDeviceID,
) -> Result<(RateListener, AliveListener), Box<dyn Error>> {
    let mut rate_listener = RateListener::new(devid, None);
    rate_listener.register()?;
    let mut alive_listener = AliveListener::new(devid);
    alive_listener.register()?;
    Ok((rate_listener, alive_listener))
}

