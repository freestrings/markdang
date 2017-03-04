extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate hyper;
extern crate regex;
extern crate rtag;
extern crate time;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use clap::App;
use regex::Regex;
use rtag::metadata::MetadataReader as Reader;
use rtag::metadata::MetadataWriter as Writer;
use rtag::metadata::Unit;
use rtag::frame::*;
use rtag::frame::types::*;
use time::PreciseTime;

use std::boxed::Box;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;
use std::path::{PathBuf, Path};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
struct All {
    file: String,
    head: Option<ViewHead>,
    frames: Option<Vec<ViewFrame>>,
    frame1: Option<Frame1>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViewHead {
    version: String,
    flags: Option<Vec<HeadFlag>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViewFrame {
    flags: Option<Vec<FrameHeaderFlag>>,
    body: FrameBody,
}

#[derive(Debug, Serialize)]
struct Simple {
    file: String,
    version: Option<String>,
    frames: Option<Vec<String>>,
    frame1: Option<Vec<String>>,
}

impl fmt::Display for All {
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

impl fmt::Display for Simple {
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

fn frame1_to_map(frame: &Frame1) -> HashMap<&str, String> {
    let mut ret = HashMap::new();

    if !frame.album.is_empty() {
        ret.insert("album", frame.album.to_string());
    }
    if !frame.artist.is_empty() {
        ret.insert("artist", frame.artist.to_string());
    }
    if !frame.comment.is_empty() {
        ret.insert("comment", frame.comment.to_string());
    }
    if !frame.genre.is_empty() {
        ret.insert("genre", frame.genre.to_string());
    }
    if !frame.title.is_empty() {
        ret.insert("title", frame.title.to_string());
    }
    if !frame.track.is_empty() {
        ret.insert("track", frame.track.to_string());
    }
    if !frame.year.is_empty() {
        ret.insert("year", frame.year.to_string());
    }

    ret
}

fn framebody_to_map<'a>(fbody: &FrameBody) -> HashMap<&'a str, String> {
    let map: Box<RefCell<HashMap<&'a str, String>>> = Box::new(RefCell::new(HashMap::new()));

    fbody.inside(|key, value| {
        let mut m = map.borrow_mut();
        m.insert(unsafe { std::mem::transmute_copy(&key) }, value);
        true
    });

    let m = map.borrow();
    m.clone()
}

fn simple<'a>(file: &'a Path,
              match_filter: &Box<Fn(HashMap<String, HashMap<&str, String>>) -> bool>)
              -> Option<Simple> {
    let reader = Reader::new(file.to_str().unwrap());

    if reader.is_err() {
        return None;
    }

    let mut simple = Simple {
        file: file.to_str().unwrap().to_string(),
        version: None,
        frames: None,
        frame1: None,
    };

    let mut bodies: HashMap<String, HashMap<&str, String>> = HashMap::new();

    for unit in reader.unwrap() {
        match unit {
            Unit::Header(head) => {
                simple.version = Some(head.version.to_string());
            }
            Unit::FrameV2(ref fhead, ref fbody) => {

                bodies.insert(fhead.id(), framebody_to_map(fbody));

                if simple.frames.is_none() {
                    simple.frames = Some(vec![]);
                }

                match simple.frames {
                    Some(ref mut f) => f.push(fhead.id()),
                    _ => {}
                };
            }
            Unit::FrameV1(frame) => {
                let map = frame1_to_map(&frame);
                let frame1 = map.keys().map(|k| k.to_string()).collect::<Vec<_>>();
                simple.frame1 = Some(frame1);
            }
            _ => {}
        }
    }

    if match_filter(bodies) {
        Some(simple)
    } else {
        None
    }

}

fn all<'a>(file: &'a Path,
           match_filter: &Box<Fn(HashMap<String, HashMap<&str, String>>) -> bool>)
           -> Option<All> {
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

    let reader = Reader::new(file.to_str().unwrap());

    if reader.is_err() {
        return None;
    }

    let mut all = All {
        file: file.to_str().unwrap().to_string(),
        head: None,
        frames: None,
        frame1: None,
    };

    let mut bodies: HashMap<String, HashMap<&str, String>> = HashMap::new();

    for unit in reader.unwrap() {
        match unit {
            Unit::Header(head) => {
                let mut flags = Vec::new();

                if head.has_flag(HeadFlag::Compression) {
                    flags.push(HeadFlag::Compression);
                }

                if head.has_flag(HeadFlag::Unsynchronisation) {
                    flags.push(HeadFlag::Unsynchronisation);
                }

                all.head = Some(ViewHead {
                    version: head.version.to_string(),
                    flags: if flags.len() > 0 { Some(flags) } else { None },
                });
            }
            Unit::FrameV2(fhead, fbody) => {

                bodies.insert(fhead.id(), framebody_to_map(&fbody));

                if all.frames.is_none() {
                    all.frames = Some(vec![]);
                }

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

                match all.frames {
                    Some(ref mut frames) => {
                        frames.push(ViewFrame {
                            flags: if flags.len() > 0 { Some(flags) } else { None },
                            body: filter_body(fbody),
                        });
                    }
                    _ => {}
                };

            }
            Unit::FrameV1(frame) => {
                all.frame1 = Some(frame);
            }
            _ => (),
        }
    }

    if match_filter(bodies) {
        Some(all)
    } else {
        None
    }
}

fn match_expr(exp: &str) -> Box<Fn(HashMap<String, HashMap<&str, String>>) -> bool> {
    use std::str::Chars;

    fn take_token(chars: &mut Chars) -> (char, String) {
        let mut tk = ' ';
        let value = chars.take_while(|ch| match ch {
                &'!' | &'^' | &'=' | &'~' | &'$' | &'.' | &'(' | &')' | &'&' | &'|' | &' ' => {
                    tk = *ch;
                    false
                }
                _ => true,
            })
            .collect();

        (tk, value)
    }

    fn take_value(tk: char, chars: &mut Chars) -> String {
        chars.take_while(|ch| *ch != tk).collect()
    }

    let mut tokens: Vec<String> = Vec::new();
    let mut chars = exp.chars();

    while let Some(ch) = chars.next() {
        match ch {
            '"' | '\'' => {
                let v = take_value(ch, &mut chars);
                tokens.push(v);
            }
            '!' | '^' | '=' | '~' | '$' | '.' | '(' | ')' | '&' | '|' => {
                tokens.push(ch.to_string());
            }
            ' ' => {
                //
            }
            _ => {
                let (tk, value) = take_token(&mut chars);
                let mut r = ch.to_string();
                r.push_str(value.as_str());
                tokens.push(r);
                if tk != ' ' {
                    tokens.push(tk.to_string());
                }
            }
        }
    }

    let mut iter = tokens.iter();
    let mut stack = Vec::new();
    let mut ordered = Vec::new();

    #[derive(Debug)]
    enum Tk {
        Del(String),
        MatchId(String),
        NotMatchId(String),
        Prop(String, String), //id, prop
        Value(String),
    }

    while let Some(token) = iter.next() {
        match token.as_str() {
            "!" | "^" | "=" | "~" | "$" | "." | "(" | "&" | "|" => {
                stack.push(Tk::Del(token.clone()))
            }
            ")" => {
                while let Some(tk) = stack.pop() {
                    match tk {
                        Tk::Del(ref v) if v == "(" => break,
                        _ => {
                            ordered.push(tk);
                        }
                    };
                }
            }
            _ => {
                match stack.pop() {
                    Some(Tk::Del(ref tk)) if tk.as_str() == "." => {
                        match stack.pop() {
                            Some(Tk::MatchId(ref id)) => {
                                stack.push(Tk::Prop(id.clone(), token.clone()));
                            }
                            _ => {
                                panic!("\n\n---------------------\n\
                                The property token \".\" is only allowed at Id token. \
                                ex) TIT1.text\n---------------------\n\n");
                            }
                        }
                    }
                    Some(Tk::Del(ref tk)) if tk.as_str() == "!" || tk.as_str() == "^" ||
                                             tk.as_str() == "=" ||
                                             tk.as_str() == "~" ||
                                             tk.as_str() == "$" => {

                        let is_prop = if let Some(&Tk::Prop(_, _)) = stack.last() {
                            true
                        } else {
                            false
                        };

                        if is_prop {
                            stack.push(Tk::Del(tk.clone()));
                            stack.push(Tk::Value(token.clone()));
                        } else if tk.as_str() == "!" {
                            stack.push(Tk::NotMatchId(token.clone()));
                        } else {
                            stack.push(Tk::MatchId(token.clone()));
                        }
                    }
                    None => {
                        stack.push(Tk::MatchId(token.clone()));
                    }
                    last @ _ => {
                        stack.push(last.unwrap());
                        stack.push(Tk::MatchId(token.clone()));
                    }
                }
            }
        }
    }

    while let Some(tk) = stack.pop() {
        ordered.insert(0, tk);
    }

    fn or(results: &mut Vec<bool>, result: bool) {
        match results.pop() {
            Some(r) => results.push(r || result),
            _ => results.push(result),
        };
    }

    fn and(results: &mut Vec<bool>, result: bool) {
        match results.pop() {
            Some(r) => results.push(r && result),
            _ => results.push(result),
        };
    }

    fn calc_result(op_stack: &mut Vec<&str>, results: &mut Vec<bool>, result: bool) {

        match op_stack.pop() {
            Some(op) => {
                match op {
                    "|" => or(results, result),
                    _ => and(results, result),
                }
            }
            _ => and(results, result),
        };
    }

    Box::new(move |frame_bodies| {

        let mut results: Vec<bool> = Vec::new();
        let mut op_stack: Vec<&str> = Vec::new();

        trace!("{:?}", ordered);
        let mut iter = ordered.iter();
        while let Some(token) = iter.next() {
            match token {
                &Tk::Prop(ref id, ref prop) => {

                    let op = iter.next();
                    let op = if let Some(&Tk::Del(ref op)) = op {
                        op.clone()
                    } else {
                        String::new()
                    };

                    let value = iter.next();
                    let value = if let Some(&Tk::Value(ref value)) = value {
                        value.clone()
                    } else {
                        String::new()
                    };

                    if op == "" || id == "" {
                        panic!("\n\n---------------------\n\
                                \"<id>.<property> (=!~$) <value>\" \
                                ex) TIT1.text~\"Dio Live\"\n---------------------\n\n");
                    }

                    let result = match frame_bodies.get(id) {
                        Some(fb) => {
                            match fb.get(prop.as_str()) {
                                Some(v) => {
                                    match op.as_str() {
                                        "=" => &value == v,
                                        "!" => &value != v,
                                        "~" => v.contains(value.as_str()),
                                        "^" => v.starts_with(value.as_str()),
                                        "$" => v.ends_with(value.as_str()),
                                        _ => false,
                                    }
                                }
                                None => false,
                            }
                        }
                        None => false,
                    };

                    calc_result(&mut op_stack, &mut results, result);
                }
                &Tk::MatchId(ref id) => {
                    calc_result(&mut op_stack, &mut results, frame_bodies.get(id).is_some());
                }
                &Tk::NotMatchId(ref id) => {
                    calc_result(&mut op_stack, &mut results, frame_bodies.get(id).is_none());
                }
                &Tk::Del(ref tk) => {
                    op_stack.push(tk);
                }
                _ => {}
            };
        }

        if results.len() != 1 {
            false
        } else {
            results[0]
        }
    })
}

fn read(matches: clap::ArgMatches) {
    let files: Vec<_> = matches.values_of("INPUT").unwrap().collect();

    let format = matches.value_of("format");

    let match_exec: Box<Fn(HashMap<String, HashMap<&str, String>>) -> bool> =
        match matches.value_of("match") {
            Some(expr) => match_expr(expr),
            _ => Box::new(|_| true),
        };

    let start = PreciseTime::now();

    for file in files {

        match PathBuf::from(file).canonicalize() {
            Ok(path) => {
                debug!("{:?}", path);

                match format {
                    Some("t") => {
                        match simple(path.as_path(), &match_exec) {
                            Some(s) => println!("{}", s),
                            _ => {}
                        };
                    }
                    Some("tt") => {
                        match all(path.as_path(), &match_exec) {
                            Some(a) => println!("{}", a),
                            _ => {}
                        };
                    }
                    Some("j") => {
                        match simple(path.as_path(), &match_exec) {
                            Some(a) => {
                                let json_str = match serde_json::to_string_pretty(&a) {
                                    Ok(s) => s,
                                    _ => "{\"err\": \"\"}".to_string(),
                                };
                                println!("{},", json_str);
                            }
                            _ => {}
                        };
                    }
                    Some("jj") => {
                        match all(path.as_path(), &match_exec) {
                            Some(a) => {
                                let json_str = match serde_json::to_string_pretty(&a) {
                                    Ok(s) => s,
                                    _ => "{\"err\": \"\"}".to_string(),
                                };
                                println!("//<");
                                println!("{}", json_str);
                                println!("//>");
                            }
                            _ => {}
                        };
                    }
                    Some("f") => {
                        match simple(path.as_path(), &match_exec) {
                            Some(_) => println!("{}", file),
                            _ => {}
                        };
                    }
                    _ => {}
                };
            }
            Err(e) => {
                panic!("{:?}", e);
            }
        }
    }

    println!("#{}", start.to(PreciseTime::now()));
}

fn write(matches: clap::ArgMatches) {
    let files: Vec<_> = matches.values_of("INPUT").unwrap().collect();

    use std::fs::File;
    use std::io::{self, BufReader, BufRead, Read};
    use std::result;

    use hyper::Url;
    use hyper::Client;

    use rtag::rw::Readable;

    #[derive(Debug, Hash, PartialEq, Eq)]
    enum WriteOption {
        Clean,
        None,
    }

    fn write(json_string: &String, options: &Option<HashSet<WriteOption>>) {
        let all: result::Result<All, serde_json::Error> =
            serde_json::from_str(json_string.as_str());

        let mut all = match all {
            Ok(all) => all,
            Err(_) => panic!("\n\n=================>Invalid json: \n{}", json_string),
        };

        let result = match options {
            &Some(ref options) => {
                if options.contains(&WriteOption::Clean) {
                    clean_write(&all)
                } else {
                    update(&mut all)
                }
            }
            _ => update(&mut all),
        };

        match result {
            Ok(_) => debug!("Write done {}", all.file),
            Err(e) => warn!("Write err {:?}", e),
        };
    }

    fn clean_write(all: &All) -> io::Result<()> {
        let file = all.file.as_str();

        debug!("clean write: {}", file);

        let writer = Writer::new(file)?;
        let frames: Vec<Unit> = match all.frames {
            Some(ref frames) => {
                let iter = frames.iter();

                iter.map(|vf| {
                        let mut frame_body = vf.body.clone();

                        if let FrameBody::APIC(ref mut frame) = frame_body {
                            match extract_url(&frame.description) {
                                Some((replaced, extraced)) => {
                                    match resource_to_bytes(extraced.as_str()) {
                                        Some(bytes) => {
                                            frame.description = replaced;
                                            frame.picture_data = bytes;
                                        }
                                        _ => {}
                                    };
                                }
                                _ => {}
                            }
                        }

                        let id = framebody_to_id(&frame_body, 4);

                        Unit::FrameV2(FrameHeader::V24(FrameHeaderV4 {
                                          id: id.to_string(),
                                          size: 0,
                                          status_flag: 0,
                                          encoding_flag: 0,
                                      }),
                                      frame_body)
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        writer.write(frames, true)
    }

    fn update(all: &mut All) -> io::Result<()> {
        let file = all.file.as_str();

        debug!("update write: {}", file);

        let writer = Writer::new(file)?;

        let (version, head_unit) = if let Some(ref vhead) = all.head {
            let version: u8 = match vhead.version.parse() {
                Ok(v) => v,
                Err(_) => 4
            };
            
            let mut head = Head {
                tag_id: "ID3".to_string(),
                version: version,
                minor_version: 0,
                flag: 0,
                size: 0
            };

            match vhead.flags {
                Some(ref flags) => {
                    for flag in flags.clone() {
                        head.set_flag(flag);
                    }
                },
                _ => {}
            };

            (version, Unit::Header(head))
        } else {
            (4, Unit::Header(Head {
                tag_id: "ID3".to_string(),
                version: 4,
                minor_version: 0,
                flag: 0,
                size: 0
            }))
        };

        let mut frames: Vec<Unit> = match all.frames {
            Some(ref frames) => {
                let iter = frames.iter();

                iter.map(|vf| {
                        let mut frame_body = vf.body.clone();

                        if let FrameBody::PIC(ref mut frame) = frame_body {
                            match extract_url(&frame.description) {
                                Some((replaced, extraced)) => {
                                    match resource_to_bytes(extraced.as_str()) {
                                        Some(bytes) => {
                                            frame.description = replaced;
                                            frame.picture_data = bytes;
                                        }
                                        _ => {}
                                    };
                                }
                                _ => {}
                            }
                        }

                        if let FrameBody::APIC(ref mut frame) = frame_body {
                            match extract_url(&frame.description) {
                                Some((replaced, extraced)) => {
                                    match resource_to_bytes(extraced.as_str()) {
                                        Some(bytes) => {
                                            frame.description = replaced;
                                            frame.picture_data = bytes;
                                        }
                                        _ => {}
                                    };
                                }
                                _ => {}
                            }
                        }

                        let id = framebody_to_id(&frame_body, version);

                        let frame_head = match version {
                            2 => FrameHeader::V22(FrameHeaderV2 {
                                id: id.to_string(),
                                size: 0
                            }),
                            3 => {
                                let mut header = FrameHeader::V23(FrameHeaderV3 {
                                    id: id.to_string(),
                                    size: 0,
                                    status_flag: 0,
                                    encoding_flag: 0
                                });

                                match vf.flags {
                                    Some(ref flags) => {
                                        for flag in flags.clone() {
                                            header.set_flag(flag);
                                        }
                                    },
                                    _ => {}
                                };

                                header
                            },
                            _ => {
                                let mut header = FrameHeader::V24(FrameHeaderV4 {
                                    id: id.to_string(),
                                    size: 0,
                                    status_flag: 0,
                                    encoding_flag: 0
                                });
                                
                                match vf.flags {
                                    Some(ref flags) => {
                                        for flag in flags.clone() {
                                            header.set_flag(flag);
                                        }
                                    },
                                    _ => {}
                                };

                                header
                            }
                        };

                        Unit::FrameV2(frame_head, frame_body)
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        if let Some(ref frame1) = all.frame1 {
            frames.push(Unit::FrameV1(frame1.clone()));
        }

        frames.insert(0, head_unit);

        writer.write(frames, false)
    }

    fn extract_url(value: &String) -> Option<(String, String)> {
        let re = Regex::new(r"#\{(.*)\}").unwrap();

        re.captures(value.as_str()).and_then(|c| {
            let cap = c.get(1).map_or("", |m| m.as_str());
            Some((re.replace(value, "").into_owned(), cap.to_string()))
        })
    }

    fn resource_to_bytes(cap_url: &str) -> Option<Vec<u8>> {

        fn file(url: Url) -> Option<Vec<u8>> {
            let path = match url.to_file_path() {
                Ok(path) => path,
                Err(e) => {
                    error!("Invalid file path: {:?}", e);
                    return None;
                }
            };

            debug!("file: {:?}", path);

            let mut fs = match File::open(path) {
                Ok(fs) => fs,
                Err(e) => {
                    error!("Can not read a file: {:?}", e);
                    return None;
                }
            };

            match fs.all_bytes() {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    error!("Unknown error: {:?}", e);
                    None
                }
            }
        }

        fn http(url: Url) -> Option<Vec<u8>> {
            debug!("http: '{}'", url);

            let client = Client::new();
            let mut response = match client.get(url).send() {
                Ok(response) => response,
                Err(e) => {
                    error!("Can not send http request: {:?}", e);
                    return None;
                }
            };

            let mut buf = vec![0u8; 1024];
            let mut dst = Vec::new();
            while let Ok(read) = response.read(&mut buf) {
                if read == 0 {
                    break;
                }
                if read < buf.len() {
                    dst.extend_from_slice(&buf[..read]);
                } else {
                    dst.extend_from_slice(&buf);
                }
            }

            Some(dst)
        }

        debug!("resource: {}", cap_url);

        let parsed_url = match Url::parse(cap_url) {
            Ok(parsed_url) => parsed_url,
            Err(_) => {
                error!("Invalid url: {}", cap_url);
                return None;
            }
        };

        if parsed_url.scheme() == "http" || parsed_url.scheme() == "https" {
            http(parsed_url)
        } else {
            file(parsed_url)
        }

    }

    fn read_option<'a>(line: String) -> Option<HashSet<WriteOption>> {
        let (_, write_option) = line.split_at(3);
        let str_options: Vec<&str> = write_option.split_whitespace().collect();

        let mut options: HashSet<WriteOption> = HashSet::new();
        for option in str_options {
            let write_option = match option.to_lowercase().as_str() {
                "clean" => WriteOption::Clean,
                _ => WriteOption::None,
            };
            options.insert(write_option);
        }

        Some(options)
    }

    for file in files {
        trace!("{}", file);

        let fs = match File::open(file) {
            Ok(fs) => fs,
            Err(_) => panic!("Can not open json file. {}", file),
        };

        let mut item = String::new();
        let mut options: Option<HashSet<WriteOption>> = None;

        let reader = BufReader::new(&fs);
        for line in reader.lines() {
            let line = line.unwrap();

            if line.starts_with("//<") {
                item.clear();
                options = read_option(line);
            } else if line.starts_with("//>") {
                write(&item, &options);
            } else {
                item.push_str(line.as_str());
                item.push_str("\n");
            }
        }

    }
}

fn main() {
    env_logger::init().unwrap();

    let matches = App::new("markdang")
        .version("0.2")
        .author("Changseok Han <freestrings@gmail.com>")
        .args_from_usage("<INPUT>... 'mp3 file pathes. ex) ./markdang file1 file2'
                          \
                          -f --format=[FORMAT] 'default value is text. (t|tt|j|jj|f) t=simple \
                          text, tt=text, j=simple json, jj=json, f=file'
                          \
                          -m --match=[MATCH] 'it find to match id. ex) -m \"!APIC | \
                          TALB.text~\'Dio\'\" see more example at README.md'

                          -w --write 'write mode on'
            ")
        .get_matches();

    if matches.is_present("write") {
        write(matches);
    } else {
        read(matches);
    }
}