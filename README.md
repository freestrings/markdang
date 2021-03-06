# What

`Markdang` is shell base application to read and to write a ID3 tag. 
- it based on **[rtag](https://github.com/freestrings/rtag) library**

# Why

To learn rust!

# Usage

## On Synology NAS

### 1. Install docker on your local machine

- https://docs.docker.com/engine/installation/


### 2. Compile

```bash
$ git clone https://github.com/freestrings/markdang.git
$ cd markdang
$ docker run --rm -it -v "$PWD":/work freestrings/rust-build-armv7
```

### 3. Upload executalbe to synology NAS

```bash
scp -P 22 "$PWD/target/armv7-unknown-linux-gnueabihf/release/markdang" \
    admin-user@your-host:/usr/local/bin/markdang
```

## On Local machine

### 1. Install Rust

- https://www.rust-lang.org/en-US/install.html

```bash
$ curl https://sh.rustup.rs -sSf | sh
```

### 2. Compile and install

```bash
$ git clone https://github.com/freestrings/markdang.git
$ cd markdang
$ cargo build --release
$ echo "export PATH=$PWD/target/release:\$PATH" > .markdang
$ source .markdang
```

## Basic usage

### --help option

```bash
$ markdang --help

markdang 0.3
Changseok Han <freestrings@gmail.com>

USAGE:
    markdang [FLAGS] [OPTIONS] <INPUT>...

FLAGS:
    -h, --help         Prints help information
    -t, --transform    ff format convert to jj format
    -V, --version      Prints version information
    -w, --write        write mode on

OPTIONS:
    -f, --format <FORMAT>    default value is text. (t|tt|j|jj|f|ff) t=simple text, tt=text, j=simple json, jj=json, f=file, ff=the absolute file path with a basic metadata
    -m, --match <MATCH>      it find to match id. ex) -m "!APIC | TALB.text~'Dio'" see more example at README.md

ARGS:
    <INPUT>...    mp3 file pathes. ex) ./markdang file1 file2

```

[Reference - properties of ID3 frames](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L946)

### Reading: -f (--format) option

- `t` simple text
- `tt` rich text
- `j` simple json
- `jj` rich json
- `f` file name only

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f tt # tt => rich text
/home/han/Musics/14.mp3
	version: 3
	TIT2(TEXT { text_encoding: ISO88591, text: "Track 14" })
	TALB(TEXT { text_encoding: ISO88591, text: "CD3" })
	TPE1(TEXT { text_encoding: ISO88591, text: "Various" })
	TRCK(TEXT { text_encoding: ISO88591, text: "14/20" })
    ...

/home/han/Musics/...mp3
..
```

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f jj # jj => rich json
//<
{
  "file": "/home/han/Musics/4.mp3",
  "head": {
    "version": "3",
    "flags": null
  },
  "frames": [
    {
      "flags": null,
      "body": {
        "TIT2": {
          "text_encoding": "ISO88591",
          "text": "Track  4"
        }
      }
    },
    {
      "flags": null,
      "body": {
        "TALB": {
          "text_encoding": "ISO88591",
          "text": "CD3"
        }
      }
    },
    ...
  ],
  "frame1": null
}
//>
//<
...
//>
```

### Find: -m (--match) option

### `!`(not) op

'!' operator can be used both for a frame id and frame property.

ex) To find that a album image is empty.

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f tt -m "\!APIC"
```

ex) To find that a album image is empty and a track is not '2/20'. 

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f tt -m "!APIC & TRCK.text\!'2/20'"
```

a `text` is property of TRCK frame above example.
 - [TRCK](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L2278)
 - [TEXT](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L1381)

###  `^`, `$`, `=`, `~` is for only property

- `^`(start with)
- `$`(end with)
- `=`(equal)
- `~`(contain)

ex) To find that a 'text_encoding' of album is 'UTF16LE' and a title contains 'Dio'

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f tt -m "TIT2.text~'Dio' & TALB.text_encoding='UTF16LE'"
```

### Complex condition

ex) A album image is empty and a artist is 'Dio' or a artist is 'Metallica'

```bash
$ find . -type f -name "*.mp3" -printf "\"%p\"\n" | xargs markdang -f tt -m "\!APIC & (TPE1~'Dio' | TPE1~'Metallica')"
```

### Write: -w (--write) flag

```bash
$ markdang ./<JSON FILE> -w
```

The format of json file. **(it is the same to `-f jj` option output)**

```text
$ markdang x.mp3 -f jj > X.txt
$ markdang X.txt -w
```

- [HeadFlag](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L1939)
- [FrameHeaderFlag](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L1916)

```
//<                                         // start
{
    "file": "",                             // <file name>   type: string, madatory
    "head": {                               // <id3 header>  type: object, optional
        "version": ""                       // type: string
        "flags": ""                         // type: string, comma seperated
    },
    "frames": [                             // <frame v2>    type: array, optional
        {
            "flags": ""                     // type: string, comma seperated
            "body": {
                "FRAME ID": {               // type: string
                    "PROPERTY": ""          // type: string
                }
            }
        },
        ...
    ],
    "frame1": {                             // <frame v1>    type: object
        "title": "",                        // type: string
        "artist": "",                       // type: string
        "album": "",                        // type: string
        "year": "",                         // type: string
        "comment": "",                      // type: string
        "track": "",                        // type: string
        "genre": ""                         // type: string
    }
}
//>                                         // end
//<
...
//>
```

### Clean writing

The meaning of 'clean writing' is remove frame1 and re-write as version 4. see a detail explain in rtag library.

- [See1](https://github.com/freestrings/rtag/blob/master/src/metadata.rs#L394)
- [See2](https://github.com/freestrings/rtag#rewrite-how-to-rewrite-a-id3-information-to-version-4)

add `clean` option in the start.

> //<clean

```json
//<clean
{
  "file": "<absolute path>/tests/v1-v2.mp3",
  "frames": [
    {
      "flags": null,
      "body": {
        "TIT2": {
          "text_encoding": "UTF8",
          "text": "타이틀"
        }
      }
    }
  ]
}
//>
```

```bash
$ markdang ./tests/clean.json -w
```

### Image

To change or add image to mp3, it must use a placeholder `#{}` in a `description` property of APIC, PIC frame.

> The https protocol does not support. because of OpenSSL version problem on ARM.

ex) 
- "description": "This description of artwork. #{file:/path/to/image.png}"
- "description": "This description of artwork. #{http://path.to/image.png}"
- [clean.json](https://github.com/freestrings/markdang/blob/test/tests/clean.json)

- [See Picture type](https://github.com/freestrings/rtag/blob/master/src/frame.rs#L1780)

```bash
{
    ...
    "frames": [
        {
        "flags": null,
        "body": {
                "APIC": {
                    "text_encoding": "UTF8",
                    "mime_type": "image/jpeg",
                    "picture_type": "CoverFront",
                    "description": "... ${URL} ...",
                    "picture_data": [],
                }
            }
        }
    ]
}
```

## Tagging Basic

### `-f ff` option and `-t` option

```bash
markdang file_path -f ff
file: /.../xxx.mp3
artwork: /.../Corver.jpg
title:
album:
artist:
band:
track:
year:

find . -name "*.mp3" -type f -printf "\%p\"\n" | xargs markdang -f ff > basic.txt

# fill the basic.txt file

markdang ./basic.txt -t > meta.txt
markdang ./meta.txt -w
```
