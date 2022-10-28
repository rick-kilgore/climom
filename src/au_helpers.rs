use core_foundation_sys::string::{
    CFStringCreateWithCString, CFStringGetCString, CFStringRef,
};
use coreaudio::audio_unit::AudioUnit;
use coreaudio::sys::{
    kAudioDevicePropertyScopeOutput, kAudioHardwareNoError,
    kAudioObjectPropertyElementMain, kCFAllocatorDefault,
    kCFStringEncodingUTF8, AudioDeviceID, AudioObjectGetPropertyData,
    AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};

use std::error::Error;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr::null;

use coreaudio::error::Error as AudioError;

const MOM_PROP_ID: &str = "momm";

pub fn identify_me_to_dad(devid: AudioDeviceID, mom_mode: &str) -> Result<(), Box<dyn Error>> {
    let mom_property_id = idFromCharsLikeAppleDoes(MOM_PROP_ID);
    println!("requesting Mom property id {}", mom_property_id);
    let mut momprop = get_mom_prop(devid, mom_property_id)?;
    let status = set_mom_prop(devid, mom_property_id, &CString::new(mom_mode)?)?;
    momprop = get_mom_prop(devid, mom_property_id)?;
    println!("devid = {}  momprop = {}", devid, momprop);
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

pub fn get_mom_prop(
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

pub fn set_mom_prop(
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
