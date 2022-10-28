use core_foundation_sys::string::{CFStringCreateWithCString, CFStringGetCString, CFStringRef};
use coreaudio::audio_unit::{SampleFormat, Scope, StreamFormat};
use coreaudio::audio_unit::audio_format::LinearPcmFlags;
use coreaudio::audio_unit::macos_helpers::{
    find_matching_physical_format, get_audio_device_ids, get_audio_device_supports_scope,
    get_device_name, get_supported_physical_stream_formats, set_device_physical_stream_format,
};
use coreaudio::error::Error as AudioError;
use coreaudio::sys::{
    kAudioDevicePropertyScopeOutput, kAudioHardwareNoError, kAudioObjectPropertyElementMain,
    kCFAllocatorDefault, kCFStringEncodingUTF8, AudioDeviceID, AudioObjectGetPropertyData,
    AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};
use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr::null;

use crate::utils::type_of;

#[derive(Copy, Clone, Debug)]
pub enum MomDeviceError<'a> {
  DADNotFound(&'a str),
  Unknown(&'a str),
}
impl <'a> std::fmt::Display for MomDeviceError<'a> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        match *self {
            MomDeviceError::DADNotFound(name_str) => write!(
                fmt,
                "Could not find the DAD on this host: searched for device with '{}'",
                name_str
            ),
            MomDeviceError::Unknown(thestr) => write!(fmt, "unknown error: {}", thestr),
        }
    }
}
impl <'a> std::error::Error for MomDeviceError<'a> {}

const MOM_PROP_ID: &str = "momm";

pub fn identify_me_to_dad(devid: AudioDeviceID, mom_mode: &str) -> Result<(), Box<dyn Error>> {
    let mom_property_id = idFromCharsLikeAppleDoes(MOM_PROP_ID);
    println!("requesting Mom property id {}", mom_property_id);
    let mut momprop = get_mom_prop_on_dev(devid, mom_property_id)?;
    let status = set_mom_prop_on_dev(devid, mom_property_id, &CString::new(mom_mode)?)?;
    momprop = get_mom_prop_on_dev(devid, mom_property_id)?;
    println!("devid = {}  momprop = {}", devid, momprop);
    Ok(())
}

pub fn find_matching_dev<'a>(
    name_substr: &'a str,
    scope: Scope,
) -> Result<(AudioDeviceID, String), Box<dyn Error + 'a>> {
    let dev_ids = get_audio_device_ids()?;
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
    return Err(Box::new(MomDeviceError::DADNotFound(name_substr)));
}

pub fn print_supported_formats(devid: AudioDeviceID) -> Result<(), Box<dyn Error>> {
    // print supported formats, disabled for now since it often crashes.
    println!("All supported formats");
    let formats = get_supported_physical_stream_formats(devid)?;
    for fmt in formats {
        println!("supported type={}\n{:?}", type_of(&fmt), &fmt);
    }

    Ok(())
}

pub fn set_physical_device_format(
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

fn idFromCharsLikeAppleDoes(chars: &str) -> u32 {
    assert!(chars.chars().count() == 4);
    let mut id: u32 = 0;
    let mut mult: u32 = 256u32.pow(3);
    for ch in chars.chars() {
        let uch = ch as u32;
        id += (uch * mult);
        mult /= 256;
    }
    return id;
}

fn get_mom_prop_on_dev(
    device_id: AudioDeviceID,
    mom_property_id: u32,
) -> Result<String, coreaudio::error::Error> {
    let property_address = AudioObjectPropertyAddress {
        mSelector: mom_property_id,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };

    macro_rules! try_status_or_return {
        ($status:expr) => {
            if $status != kAudioHardwareNoError as i32 {
                return Err(AudioError::Unknown($status));
            }
        };
    }

    let device_name: CFStringRef = null();
    let data_size = std::mem::size_of::<CFStringRef>();
    let c_str = unsafe {
        let c_string: *const c_char = null();
        let status = AudioObjectGetPropertyData(
            device_id,
            &property_address as *const _,
            0,
            null(),
            &data_size as *const _ as *mut _,
            &device_name as *const _ as *mut _,
        );
        try_status_or_return!(status);
        let mut buf: [i8; 255] = [0; 255];
        let result = CFStringGetCString(
            device_name,
            buf.as_mut_ptr(),
            buf.len() as _,
            kCFStringEncodingUTF8,
        );
        if result == 0 {
            return Err(AudioError::Unknown(result as i32));
        }
        let name: &CStr = CStr::from_ptr(buf.as_ptr());
        return Ok(name.to_str().unwrap().to_owned());
        CStr::from_ptr(c_string as *mut _)
    };
    Ok(c_str.to_str().unwrap().to_owned())
}

fn set_mom_prop_on_dev(
    device_id: AudioDeviceID,
    mom_property_id: u32,
    mom_mode: &CString,
) -> Result<bool, coreaudio::error::Error> {
    let property_address = AudioObjectPropertyAddress {
        mSelector: mom_property_id,
        mScope: kAudioDevicePropertyScopeOutput,
        mElement: kAudioObjectPropertyElementMain,
    };

    macro_rules! try_status_or_return {
        ($status:expr) => {
            if $status != kAudioHardwareNoError as i32 {
                return Err(AudioError::Unknown($status));
            }
        };
    }

    let c_str = unsafe {
        println!("calling CFStringCreateWithCString()");
        let device_name = CFStringCreateWithCString(
            kCFAllocatorDefault as *const c_void,
            mom_mode.as_ptr() as *const c_char,
            kCFStringEncodingUTF8,
        );
        let data_size = std::mem::size_of::<CFStringRef>();
        println!(
            "calling AudioObjectSetPropertyData() with size {}",
            data_size
        );
        let status = AudioObjectSetPropertyData(
            device_id,
            &property_address as *const _,
            0,
            null(),
            data_size as u32,
            &device_name as *const _ as *mut _,
        );
        try_status_or_return!(status);
        println!("DONE calling AudioObjectSetPropertyData()");
        return Ok(true);
    };
    Ok(true)
}
