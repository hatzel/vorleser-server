use ffmpeg::*;

use std::mem;
use std::ffi::{CStr, CString};
use std::ptr;
use std::path::{Path, PathBuf};
use std::slice;
use super::util::*;
use std::collections::HashMap;
use std::fmt::{Formatter, Debug};
use std::str::Split;
use worker::error::*;
use std::fmt;
use std::error;
use std::result;
use std::fs::File;
use std::io::Write;

#[derive(PartialEq, Eq, Debug)]
pub enum ImageType {
    PNG,
    JPG
}

pub struct Image {
    pub data: Vec<u8>,
    pub image_type: ImageType
}

#[derive(Debug)]
pub struct MediaInfo {
    pub length: f64,
    pub chapters: Vec<Chapter>,
    pub title: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub title: Option<String>,
    pub metadata: HashMap<String, String>,
    pub start: f64,
}

impl Image {
    pub fn save(&self, path: &AsRef<Path>) -> Result<()> {
        let mut file = File::create(path)?;
        file.write_all(&self.data[..])?;
        Ok(())
    }
}

impl Chapter {
    fn from_av_chapter(av: &AVChapter) -> Chapter {
        let start = apply_timebase(av.start, &av.time_base);
        let d = dict_to_map(av.metadata as *mut Dictionary);
        let title = d.get("title").cloned();
        Chapter {
            start: start,
            title: title,
            metadata: d,
        }
    }

    fn from_av_chapters(avs: &[&AVChapter]) -> Vec<Chapter> {
        let mut res = Vec::new();
        for av in avs.iter() {
            res.push(Self::from_av_chapter(av))
        }
        res
    }
}

pub struct Format<'a> {
    name: &'a str,
    mime_type: &'a str,
    extensions: Split<'a, char>,
    flags: i32,
    codec: &'a Codec
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
struct Codec {
    id: AVCodecID,
    tag: usize
}

pub struct MediaFile {
    ctx: *mut AVFormatContext,
    path: PathBuf,
    averror: i32,
    av_packet: Option<AVPacket>,
}

impl Debug for MediaFile {
    fn fmt(&self, f: &mut Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "Mediafile for {:?}", self.path)
    }
}

impl MediaFile {
    pub fn read_file(file_name: &Path) -> Result<Self> {
        let file_name_str = match file_name.to_str() {
            Some(s) => s,
            None => return Err(ErrorKind::InvalidUtf8.into())
        };
        unsafe {
            ensure_av_register_all();
            let c_file_name = CString::new(file_name_str).expect("Null byte in filename.");
            let mut new = Self {
                path: file_name.to_owned(),
                ctx: avformat_alloc_context(),
                averror: 0,
                av_packet: None,
            };
            new.averror = try!(check_av_result(avformat_open_input(
                &mut new.ctx,
                c_file_name.as_ptr(),
                ptr::null(),
                ptr::null_mut()
            )));
            try!(check_av_result(avformat_find_stream_info(new.ctx, ptr::null_mut())));
            Ok(new)
        }
    }

    pub fn probe_format(&self) -> Result<()> {
        let probesize = 5000000;
        let mut buf: Vec<u8> = Vec::with_capacity((probesize + AVPROBE_PADDING_SIZE) as usize);
        for x in 0..(probesize + AVPROBE_PADDING_SIZE) {
            buf.push(0)
        };
        let c_file_name = CString::new(self.path.to_str()
                                       .unwrap_or({
                                           return Err(ErrorKind::InvalidUtf8.into())
                                       })).expect("Null byte in filename");
        let mut probe_data = AVProbeData {
            filename: c_file_name.as_ptr(),
            buf: buf.as_mut_ptr(),
            buf_size: probesize,
            mime_type: ptr::null()
        };
        unsafe {
            av_probe_input_format(&mut probe_data, 1);
        };
        Ok(())
    }

    pub fn guess_format<'a>(&'a self) -> Format {
        unsafe{
            let iformat = &(*(*self.ctx).iformat);
            Format {
                name: CStr::from_ptr(iformat.name).to_str().unwrap(),
                flags: iformat.flags,
                extensions: CStr::from_ptr(iformat.name).to_str().unwrap().split(','),
                mime_type: CStr::from_ptr(iformat.mime_type).to_str().unwrap(),
                codec: &*(*iformat.codec_tag as *const Codec),
            }
        }
    }

    pub fn read_packet(&self) -> Result<Option<AVPacket>> {
        unsafe {
            let mut pkt = mem::uninitialized::<AVPacket>();
            match check_av_result(av_read_frame(self.ctx, &mut pkt)) {
                Err(Error(ErrorKind::MediaError(_, AVERROR_EOF), _)) => Ok(None),
                Err(e) => Err(e),
                _ => Ok(Some(pkt))
            }
        }
    }

    pub fn get_coverart(self) -> Result<Option<Image>> {
        unsafe {
            let best_image = try!(self.get_best_stream(AVMEDIA_TYPE_VIDEO));
            let codec = (*best_image.codecpar).codec_id;
            loop {
                match try!(self.read_packet()) {
                    Some(ref pkt) => {
                        let image_type = match codec {
                            AV_CODEC_ID_PNG => ImageType::PNG,
                            AV_CODEC_ID_MJPEG => ImageType::JPG,
                            _ => return Ok(None)
                        };
                        if pkt.stream_index == best_image.index {
                            return Ok(Some(
                                Image {
                                    image_type: image_type,
                                    data: slice::from_raw_parts(
                                        pkt.data, pkt.size as usize
                                        ).to_owned()
                                }))
                        } else {
                            continue;
                        }
                    },
                    None => break
                }
            };
            Ok(None)
        }
    }

    pub fn get_chapters(&self) -> Vec<Chapter> {
        Chapter::from_av_chapters(self.av_chapter_slice())
    }

    fn av_chapter_slice(&self) -> &[&AVChapter] {
        unsafe {
            slice::from_raw_parts(
                mem::transmute((*self.ctx).chapters),
                (*self.ctx).nb_chapters as usize
                )
        }
    }

    pub fn get_mediainfo(&self) -> MediaInfo {
        unsafe {
            let md = dict_to_map((*self.ctx).metadata as *mut Dictionary);
            MediaInfo {
                title: md.get("title").unwrap_or(
                    &(*self.path.file_name().unwrap().to_string_lossy()).to_owned()
                ).to_owned(),
                chapters: self.get_chapters(),
                length: apply_timebase((*self.ctx).duration, &AV_TIME_BASE_Q),
                metadata: md
            }
        }
    }
}

impl MediaFile {
    pub fn get_streams(&self) -> &[&AVStream] {
        unsafe {
            slice::from_raw_parts(mem::transmute((*self.ctx).streams), (*self.ctx).nb_streams as usize)
        }
    }

    pub fn get_best_stream(&self, media_type: AVMediaType) -> Result<&AVStream> {
        unsafe {
            // for s in self.get_streams() {
            //     if (*(*s).codecpar).codec_type == AVMEDIA_TYPE_AUDIO {
            //         println!("{:?}", (*s).index);
            //         return Some(s)
            //     }
            // }
            let stream_index = try!(check_av_result(av_find_best_stream(self.ctx, media_type, -1, -1, ptr::null_mut(), 0)));
            Ok(self.get_streams()[stream_index as usize])
        }
    }
}

impl Drop for MediaFile {
    fn drop(&mut self) {
        if self.averror == 0 {
            unsafe {
                avformat_close_input(&mut self.ctx);
                if let Some(ref mut pkt) = self.av_packet {
                    av_free_packet(pkt);
                }
                avformat_free_context(self.ctx);
            }
        }
    }
}
