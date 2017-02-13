extern crate rtag;
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use clap::App;
use rtag::metadata::MetadataReader as Reader;
use rtag::metadata::Unit;
use rtag::frame::{HeadFlag, FlagAware, FrameBody};

use std::vec::Vec;

#[derive(Debug, Serialize)]
struct Id3 {
    file: String,
    head: Option<Head>,
    frames: Option<Vec<FrameBody>>
}

#[derive(Debug, Serialize)]
struct Head {
    version: u8,
    flags: Option<Vec<HeadFlag>>
}

fn main() {
    let matches = App::new("automarkddang")
        .version("0.1")
        .author("Changseok Han <freestrings@gmail.com>")
        .args_from_usage("<INPUT>... 'mp3 file pathes. ex) ./automarkddang file1 file2'")
        .get_matches();

    let files = matches.values_of("INPUT").unwrap();

    for file in files {

        let mut head = None;
        let mut frames = Vec::new();

        match Reader::new(file) {
            Ok(reader) => {
                for u in reader {
                    match u {
                        Unit::Header(_head) => {
                            let mut flags = Vec::new();

                            if _head.has_flag(HeadFlag::Compression) {
                                flags.push(HeadFlag::Compression);
                            }

                            if _head.has_flag(HeadFlag::Unsynchronisation) {
                                flags.push(HeadFlag::Unsynchronisation);
                            }

                            head = Some(Head {
                                version: _head.version,
                                flags: if flags.len() > 0 { Some(flags) } else { None }
                            });

                        },
                        Unit::FrameV2(_, fbody) => {
                            frames.push(fbody);
                        }
                        _ => (),
                    }
                }
            }
            _ => println!("Invalid file '{}'", file),
        }

        let id3 = Id3 {
            file: file.to_owned(),
            head: head,
            frames: if frames.len() > 0 { Some(frames) } else { None }
        };

        let j = serde_json::to_string(&id3).unwrap();
        println!("{}", j);
    }
}