extern crate clap;
extern crate rtag;
extern crate time;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use clap::App;
use rtag::metadata::MetadataReader as Reader;
use rtag::metadata::Unit;
use rtag::frame;
use rtag::frame::*;
use time::PreciseTime;

use std::fmt;
use std::str;
use std::vec::Vec;

#[derive(Serialize)]
struct All<'a> {
    file: &'a str,
    head: Option<Head>,
    frames: Option<Vec<Frame>>,
    frame1: Option<Frame1>,
}

impl<'a> fmt::Display for All<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(f, "{}\n", self.file);

        match self.head {
            Some(ref v) => {
                let _ = write!(f, "\tversion: {}\n", v.version);
                if let Some(ref v) = v.flags {
                    let _ = write!(f, "\tflags: {:?}\n", v);
                }
            }
            _ => (),
        };

        match self.frames {
            Some(ref vv) => {
                for v in vv {
                    if let Some(ref v) = v.flags {
                        let _ = write!(f, "\tframe_flags: {:?}\n", v);
                    }
                    let _ = write!(f, "\t{:?}\n", v.body);
                }
            }
            _ => (),
        };

        match self.frame1 {
            Some(ref v) => {
                let _ = write!(f, "\n{:?}", v);
            }
            _ => (),
        };

        Ok(())
    }
}

#[derive(Serialize)]
struct Head {
    version: String,
    flags: Option<Vec<HeadFlag>>,
}

#[derive(Serialize)]
struct Frame {
    flags: Option<Vec<FrameHeaderFlag>>,
    body: FrameBody,
}

#[derive(Serialize)]
struct Simple<'a> {
    file: &'a str,
    version: Option<String>,
    frames: Option<Vec<String>>,
    frame1: Option<Vec<String>>,
}

impl<'a> fmt::Display for Simple<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let _ = write!(f, "{}, ", self.file);

        match self.version {
            Some(ref v) => {
                let _ = write!(f, "version: {}, ", v);
            }
            _ => (),
        };

        match self.frames {
            Some(ref v) => {
                let _ = write!(f, "frames: {:?}, ", v);
            }
            _ => (),

        };

        match self.frame1 {
            Some(ref v) => {
                let _ = write!(f, "v1: {:?},", v);
            }
            _ => (),
        }

        Ok(())
    }
}

fn collect
    (file: &str,
     filter: &Vec<&str>,
     not_filter: &Vec<&str>)
     -> Option<(Option<frame::Head>, Option<Vec<(FrameHeader, FrameBody)>>, Option<Frame1>)> {
    let mut head = None;
    let mut frames = Vec::new();
    let mut frame1 = None;

    fn start_with_in(id: &str, filter: &Vec<&str>) -> bool {
        for f in filter {
            return match id.find(f) {
                Some(idx) => idx == 0,
                _ => false,
            };
        }

        false
    }

    fn filter_fn(frames: &Vec<(FrameHeader, FrameBody)>, filter: &Vec<&str>) -> bool {
        for frame in frames {
            let &(ref fhead, _) = frame;
            let id = match fhead {
                &FrameHeader::V22(ref head) => head.id.clone(),
                &FrameHeader::V23(ref head) => head.id.clone(),
                &FrameHeader::V24(ref head) => head.id.clone(),
            };

            let id = id.as_str();

            if start_with_in(id, filter) {
                return true;
            }
        }

        false
    }

    match Reader::new(file) {
        Ok(reader) => {
            for unit in reader {
                match unit {
                    Unit::Header(_head) => head = Some(_head),
                    Unit::FrameV2(_fhead, _fbody) => frames.push((_fhead, _fbody)),
                    Unit::FrameV1(_frame) => frame1 = Some(_frame),
                    _ => (),
                }
            }
        }
        _ => (),
    };

    if filter.len() > 0 {
        if !filter_fn(&frames, &filter) {
            return None;
        }
    }

    if not_filter.len() > 0 {
        if filter_fn(&frames, &not_filter) {
            return Some((head, if frames.len() > 0 { Some(frames) } else { None }, frame1));
        }
    }

    Some((head, if frames.len() > 0 { Some(frames) } else { None }, frame1))
}

fn filter_body(_body: FrameBody) -> FrameBody {
    match _body {
        FrameBody::PIC(ref body) => {
            FrameBody::PIC(PIC {
                text_encoding: body.text_encoding.clone(),
                image_format: body.image_format.clone(),
                picture_type: body.picture_type.clone(),
                description: body.description.clone(),
                picture_data: Vec::new(),
            })
        }
        FrameBody::APIC(ref body) => {
            FrameBody::APIC(APIC {
                text_encoding: body.text_encoding.clone(),
                mime_type: body.mime_type.clone(),
                picture_type: body.picture_type.clone(),
                description: body.description.clone(),
                picture_data: Vec::new(),
            })
        }
        _ => _body,
    }
}

fn all<'a>(file: &'a str, filter: &Vec<&str>, not_filter: &Vec<&str>) -> Option<All<'a>> {
    let collected = collect(file, filter, not_filter);

    if collected.is_none() {
        return None;
    }

    let (head, frames, frame1) = collected.unwrap();

    let head = if head.is_some() {
        let head = head.unwrap();
        let mut flags = Vec::new();

        if head.has_flag(HeadFlag::Compression) {
            flags.push(HeadFlag::Compression);
        }

        if head.has_flag(HeadFlag::Unsynchronisation) {
            flags.push(HeadFlag::Unsynchronisation);
        }

        Some(Head {
            version: head.version.to_string(),
            flags: if flags.len() > 0 { Some(flags) } else { None },
        })
    } else {
        None
    };

    let frames = if frames.is_some() {
        let frames = frames.unwrap()
            .into_iter()
            .fold(Vec::new(), |mut vec, frame| {
                let frame = match frame {
                    (FrameHeader::V22(_), fbody) => {
                        Frame {
                            flags: None,
                            body: fbody,
                        }
                    }
                    (FrameHeader::V23(fhead), fbody) => {
                        let mut flags = Vec::new();

                        if fhead.has_flag(FrameHeaderFlag::Compression) {
                            flags.push(FrameHeaderFlag::Compression);
                        }

                        if fhead.has_flag(FrameHeaderFlag::Encryption) {
                            flags.push(FrameHeaderFlag::Encryption);
                        }

                        if fhead.has_flag(FrameHeaderFlag::FileAlter) {
                            flags.push(FrameHeaderFlag::FileAlter);
                        }

                        if fhead.has_flag(FrameHeaderFlag::GroupIdentity) {
                            flags.push(FrameHeaderFlag::GroupIdentity);
                        }

                        if fhead.has_flag(FrameHeaderFlag::ReadOnly) {
                            flags.push(FrameHeaderFlag::ReadOnly);
                        }

                        if fhead.has_flag(FrameHeaderFlag::TagAlter) {
                            flags.push(FrameHeaderFlag::TagAlter);
                        }

                        Frame {
                            flags: if flags.len() > 0 { Some(flags) } else { None },
                            body: filter_body(fbody),
                        }
                    }
                    (FrameHeader::V24(fhead), fbody) => {
                        let mut flags = Vec::new();

                        if fhead.has_flag(FrameHeaderFlag::Compression) {
                            flags.push(FrameHeaderFlag::Compression);
                        }

                        if fhead.has_flag(FrameHeaderFlag::Encryption) {
                            flags.push(FrameHeaderFlag::Encryption);
                        }

                        if fhead.has_flag(FrameHeaderFlag::FileAlter) {
                            flags.push(FrameHeaderFlag::FileAlter);
                        }

                        if fhead.has_flag(FrameHeaderFlag::GroupIdentity) {
                            flags.push(FrameHeaderFlag::GroupIdentity);
                        }

                        if fhead.has_flag(FrameHeaderFlag::ReadOnly) {
                            flags.push(FrameHeaderFlag::ReadOnly);
                        }

                        if fhead.has_flag(FrameHeaderFlag::TagAlter) {
                            flags.push(FrameHeaderFlag::TagAlter);
                        }

                        if fhead.has_flag(FrameHeaderFlag::DataLength) {
                            flags.push(FrameHeaderFlag::DataLength);
                        }

                        if fhead.has_flag(FrameHeaderFlag::Unsynchronisation) {
                            flags.push(FrameHeaderFlag::Unsynchronisation);
                        }

                        Frame {
                            flags: if flags.len() > 0 { Some(flags) } else { None },
                            body: filter_body(fbody),
                        }
                    }
                };

                vec.push(frame);

                vec
            });

        if frames.len() > 0 { Some(frames) } else { None }
    } else {
        None
    };

    Some(All {
        file: file,
        head: head,
        frames: frames,
        frame1: frame1,
    })
}

fn simple<'a>(file: &'a str, filter: &Vec<&str>, not_filter: &Vec<&str>) -> Option<Simple<'a>> {
    let collected = collect(file, filter, not_filter);

    if collected.is_none() {
        return None;
    }

    let (head, frames, frame1) = collected.unwrap();

    let version = if head.is_some() {
        Some(head.unwrap().version.to_string())
    } else {
        None
    };

    let frame_ids = if frames.is_some() {
        Some(frames.unwrap()
            .into_iter()
            .map(|(fhead, _)| match fhead {
                FrameHeader::V22(fhead) => fhead.id,
                FrameHeader::V23(fhead) => fhead.id,
                FrameHeader::V24(fhead) => fhead.id,
            })
            .collect::<Vec<String>>())
    } else {
        None
    };

    let frame1_ids = if frame1.is_some() {
        let frame = frame1.unwrap();

        let mut ret = Vec::new();
        if !frame.album.is_empty() {
            ret.push("album".to_string());
        }
        if !frame.artist.is_empty() {
            ret.push("artist".to_string());
        }
        if !frame.comment.is_empty() {
            ret.push("comment".to_string());
        }
        if !frame.genre.is_empty() {
            ret.push("genre".to_string());
        }
        if !frame.title.is_empty() {
            ret.push("title".to_string());
        }
        if !frame.track.is_empty() {
            ret.push("track".to_string());
        }
        if !frame.year.is_empty() {
            ret.push("year".to_string());
        }

        Some(ret)
    } else {
        None
    };

    Some(Simple {
        file: file,
        version: version,
        frames: frame_ids,
        frame1: frame1_ids,
    })
}

fn json(format: Format, files: Vec<&str>, filter: &Vec<&str>, not_filter: &Vec<&str>) {
    let start = PreciseTime::now();
    println!("[");
    for file in files {
        match format {
            Format::Simple => {
                if let Some(s) = simple(file, filter, not_filter) {
                    let json_str = match serde_json::to_string(&s) {
                        Ok(s) => s,
                        _ => "{\"err\": \"\"}".to_string(),
                    };
                    println!("{},", json_str);
                }
            }
            Format::SuperSimple => {
                if let Some(_) = simple(file, filter, not_filter) {
                    println!("\"{}\",", file);
                }
            }
            _ => {
                if let Some(s) = all(file, filter, not_filter) {
                    let json_str = match serde_json::to_string(&s) {
                        Ok(s) => s,
                        _ => "{\"err\": \"\"}".to_string(),
                    };

                    println!("{},", json_str);
                }
            }
        };
    }
    println!("\"{}\"", start.to(PreciseTime::now()));
    println!("]");
}

fn text(format: Format, files: Vec<&str>, filter: &Vec<&str>, not_filter: &Vec<&str>) {
    let start = PreciseTime::now();
    for file in files {
        match format {
            Format::Simple => {
                if let Some(s) = simple(file, filter, not_filter) {
                    println!("{}", s);
                }
            }
            Format::SuperSimple => {
                if let Some(_) = simple(file, filter, not_filter) {
                    println!("\"{}\"", file);
                }
            }
            _ => {
                if let Some(s) = all(file, filter, not_filter) {
                    println!("{}", s);
                }
            }
        };
    }
    println!("{}", start.to(PreciseTime::now()));
}

#[derive(Debug)]
enum Format {
    Simple,
    SuperSimple,
    All,
}

fn main() {
    let matches = App::new("automarkddang")
        .version("0.1")
        .author("Changseok Han <freestrings@gmail.com>")
        .args_from_usage("<INPUT>... 'mp3 file pathes. ex) ./automarkddang file1 file2'
                          \
                          -t --text 'print as text format. if not given, print as json'
                          -s... --simple 'print as simple imformation. if not given, print all'
                          -m \
                          --match=[MATCH] 'it find to match id. and it support comma \
                          seperated multiple id ex) -m TIT2,TALB and !(not) operator also. -m !T'
            ")
        .get_matches();

    let files: Vec<_> = matches.values_of("INPUT").unwrap().collect();

    let filter: Vec<&str> = if matches.is_present("match") {
        let id_str = matches.value_of("match").unwrap();
        id_str.split(",").collect()
    } else {
        Vec::new()
    };

    let not_filter: Vec<&str> = filter.clone()
        .iter()
        .filter(|i| i.find("!") == Some(0))
        .map(|i| {
            let (_, last) = i.split_at(1);
            last
        })
        .collect();

    let filter: Vec<&str> = filter.iter()
        .filter(|i| i.find("!") == None)
        .map(|i| *i)
        .collect();

    let mut format = if matches.is_present("simple") {
        Format::Simple
    } else {
        Format::All
    };

    if matches.occurrences_of("simple") > 1 {
        format = Format::SuperSimple
    }

    if matches.is_present("text") {
        text(format, files, &filter, &not_filter);
    } else {
        json(format, files, &filter, &not_filter);
    }

}