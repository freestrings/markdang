extern crate clap;
extern crate rtag;
extern crate time;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use clap::App;
use rtag::metadata::MetadataReader as Reader;
use rtag::metadata::Unit;
use rtag::frame::*;
use rtag::frame::types::*;
use time::PreciseTime;

use std::fmt;
use std::str;
use std::collections::{HashMap, HashSet};

#[derive(Serialize)]
struct All<'a> {
    file: &'a str,
    head: Option<ViewHead>,
    frames: Option<Vec<ViewFrame>>,
    frame1: Option<Frame1>,
}

#[derive(Serialize)]
struct ViewHead {
    version: String,
    flags: Option<Vec<HeadFlag>>,
}

#[derive(Serialize)]
struct ViewFrame {
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

fn simple<'a>(file: &'a str, match_filter: &Box<Fn(String, &FrameBody) -> bool>) -> Option<Simple<'a>> {
    let reader = Reader::new(file);

    if reader.is_err() {
        return None;
    }

    let mut simple = Simple {
        file: file,
        version: None,
        frames: None,
        frame1: None,
    };

    for unit in reader.unwrap() {
        match unit {
            Unit::Header(head) => {
                simple.version = Some(head.version.to_string());
            }
            Unit::FrameV2(fhead, _) => {
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

    Some(simple)
}

fn all<'a>(file: &'a str, match_filter: &Box<Fn(String, &FrameBody) -> bool>) -> Option<All<'a>> {
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

    let reader = Reader::new(file);

    if reader.is_err() {
        return None;
    }

    let mut all = All {
        file: file,
        head: None,
        frames: None,
        frame1: None,
    };

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

    Some(all)
}

fn match_expr(exp: &str) -> Box<Fn(String, &FrameBody) -> bool> {
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

    Box::new(move |id, frame_body| {

        fn calc_result(op_stack: &mut Vec<&str>, results: &mut Vec<bool>, result: bool) {
            match op_stack.pop() {
                Some(op) => {
                    match op {
                        "|" => {
                            match results.pop() {
                                Some(r) => results.push(r || result),
                                _ => results.push(result)
                            }
                        }
                        _ => {
                            match results.pop() {
                                Some(r) => results.push(r && result),
                                _ => results.push(result)
                            }
                        }
                    }
                },
                _ => {
                    match results.pop() {
                        Some(r) => results.push(r && result),
                        _ => results.push(result)
                    }
                }
            };
        }

        let properties = frame_body.to_map();
        if properties.is_err()  {
            return false;
        }

        let properties = properties.unwrap();
        let keys: HashSet<String> = properties.keys().map(|k| k.to_string()).collect();

        let mut results: Vec<bool> = Vec::new();
        let mut op_stack: Vec<&str> = Vec::new();

        let mut iter = ordered.iter();
        while let Some(token) = iter.next() {

            match token {
                &Tk::Prop(ref match_id, ref prop) => {

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

                    if op == "" || match_id == "" {
                        panic!("\n\n---------------------\n\
                                \"<id>.<property> (=!~$) <value>\" \
                                ex) TIT1.text~\"Dio Live\"\n---------------------\n\n");
                    }

                    let result = match_id == &id && match properties.get(prop.as_str()) {
                        Some(v) => {
                            match op.as_str() {
                                "=" => &value == v,
                                "~" => v.contains(value.as_str()),
                                "^" => v.starts_with(value.as_str()),
                                "$" => v.ends_with(value.as_str()),
                                _ => false
                            }
                        },
                        None => false
                    };

                    calc_result(&mut op_stack, &mut results, result);
                }
                &Tk::MatchId(ref match_id) => {
                    calc_result(&mut op_stack, &mut results, &id == match_id);
                }
                &Tk::NotMatchId(ref match_id) => {
                    calc_result(&mut op_stack, &mut results, !keys.contains(match_id));
                }
                &Tk::Del(ref tk) => {
                    op_stack.push(tk);
                }
                _ => {}
            };
        };

        if results.len() != 1 {
            false
        } else {
            results[0]
        }
    })
}

fn main() {
    let matches = App::new("automarkddang")
        .version("0.1")
        .author("Changseok Han <freestrings@gmail.com>")
        .args_from_usage("<INPUT>... 'mp3 file pathes. ex) ./automarkddang file1 file2'
                          \
                          -f --format=[FORMAT] 'default value is text. (t|tt|j|jj|f) t=simple \
                          text, tt=text, j=simple json, jj=json, f=file'
                          \
                          -m --match=[MATCH] 'it find to match id. and it support comma \
                          seperated multiple id ex) -m TIT2,TALB and !(not) operator also. -m \
                          !T'
            ")
        .get_matches();

    let files: Vec<_> = matches.values_of("INPUT").unwrap().collect();

    let format = matches.value_of("format");

    let match_exec: Box<Fn(String, &FrameBody) -> bool> = match matches.value_of("match") {
        Some(expr) => match_expr(expr),
        _ => Box::new(|String, frame_body| true),
    };

    let start = PreciseTime::now();

    match format {
        Some("j") | Some("jj") => {
            println!("[");
        }
        _ => {}
    };

    for file in files {
        match format {
            Some("t") => {
                match simple(file, &match_exec) {
                    Some(s) => println!("{}", s),
                    _ => {}
                };
            }
            Some("tt") => {
                match all(file, &match_exec) {
                    Some(a) => println!("{}", a),
                    _ => {}
                };
            }
            Some("j") => {
                match simple(file, &match_exec) {
                    Some(a) => {
                        let json_str = match serde_json::to_string(&a) {
                            Ok(s) => s,
                            _ => "{\"err\": \"\"}".to_string(),
                        };
                        println!("\t{},", json_str);
                    }
                    _ => {}
                };
            }
            Some("jj") => {
                match all(file, &match_exec) {
                    Some(a) => {
                        let json_str = match serde_json::to_string(&a) {
                            Ok(s) => s,
                            _ => "{\"err\": \"\"}".to_string(),
                        };
                        println!("\t{},", json_str);
                    }
                    _ => {}
                };
            }
            Some("f") => {}
            _ => {}
        };
    }

    match format {
        Some("t") | Some("tt") => {
            println!("{}", start.to(PreciseTime::now()));
        }
        Some("j") | Some("jj") => {
            println!("\t\"{}\"", start.to(PreciseTime::now()));
            println!("]");
        }
        _ => {}
    };
}