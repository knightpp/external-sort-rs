use itertools::Itertools;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "generator")]
/// Sort
struct Config {
    /// Input file
    #[structopt(short = "i", long, parse(from_os_str))]
    input: PathBuf,
    /// Output file
    #[structopt(short = "o", long, parse(from_os_str), default_value = "sorted.txt")]
    output: PathBuf,
}

/// Buffer size in bytes
const BUFFER_SIZE_IN_BYTES: usize = 1024 * 1024 * 100; // 100 MiB

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn main() -> Result<()> {
    let conf: Config = Config::from_args();
    // let conf = Config {
    //     input: PathBuf::from("gen.txt"),
    //     output: PathBuf::from("out.txt"),
    // };
    let start = std::time::Instant::now();
    let mut files = divide_files(&conf.input)?;
    //dbg!(start.elapsed().as_secs_f64());
    while files.len() > 1 {
        let a = files.pop_front().unwrap();
        let b = files.pop_front().unwrap();
        //let start = std::time::Instant::now();
        files.push_back(merge_and_delete((&a, &b))?);
        //dbg!(start.elapsed().as_secs_f64());
    }

    let last = files.pop_front().unwrap();
    let tmp = last.with_extension("TEMP_FILE");
    binary_f64_to_strings(&last, &tmp)?;
    std::fs::remove_file(&last)?;
    std::fs::rename(tmp, &conf.output)?;
    dbg!(start.elapsed().as_secs_f64());
    Ok(())
}

/// Take binary file that actually is a f64 slice bytes
/// and convert it to f64 strings.
fn binary_f64_to_strings(binary_file: &Path, output: &Path) -> Result<()> {
    const TYPE_SIZE: usize = 8;

    let br = BufReader::with_capacity(BUFFER_SIZE_IN_BYTES / 2, File::open(binary_file)?);
    let mut bw = BufWriter::with_capacity(BUFFER_SIZE_IN_BYTES / 2, File::create(output)?);

    let iter = br.bytes().map(|x| x.unwrap()).chunks(TYPE_SIZE);

    let iter = iter
        .borrow()
        .into_iter()
        .map(|chunk| bytes_to_f64(chunk.collect_vec()));

    for num in iter {
        bw.write_fmt(format_args!("{}\n", lexical::to_string(num)))?;
    }
    Ok(())
}

/// Merge two files into one (with name of files.0) and delete files.1
fn merge_and_delete(files: (&Path, &Path)) -> Result<PathBuf> {
    let path = PathBuf::from("mergeanddelete.tmp");
    {
        const TYPE_SIZE: usize = 8;
        let mut out_file = BufWriter::with_capacity(BUFFER_SIZE_IN_BYTES / 2, File::create(&path)?);
        let file_a = BufReader::with_capacity(BUFFER_SIZE_IN_BYTES / 4, File::open(files.0)?);
        let file_b = BufReader::with_capacity(BUFFER_SIZE_IN_BYTES / 4, File::open(files.1)?);

        let iter_a = file_a.bytes().map(|x| x.unwrap()).chunks(TYPE_SIZE);
        let iter_b = file_b.bytes().map(|x| x.unwrap()).chunks(TYPE_SIZE);

        let mut iter_a = iter_a
            .borrow()
            .into_iter()
            .map(|chunk| bytes_to_f64(chunk.collect_vec()));
        let mut iter_b = iter_b
            .borrow()
            .into_iter()
            .map(|chunk| bytes_to_f64(chunk.collect_vec()));

        let mut fa: Option<f64> = iter_a.next();
        let mut fb: Option<f64> = iter_b.next();
        loop {
            if let None = fa {
                break;
            }
            if let None = fb {
                break;
            }

            if let Ordering::Less = fa.partial_cmp(&fb).unwrap() {
                // <
                out_file.write_all(bytes_of_f64(&fa.unwrap()))?;
                fa = iter_a.next();
            } else {
                // >=
                out_file.write_all(bytes_of_f64(&fb.unwrap()))?;
                fb = iter_b.next();
            }
        }
        while let Some(x) = iter_a.next() {
            // a is not exhausted
            out_file.write_all(bytes_of_f64(&x))?;
        }
        while let Some(x) = iter_b.next() {
            // b is not exhausted
            out_file.write_all(bytes_of_f64(&x))?;
        }
    }

    std::fs::remove_file(files.0)?;
    std::fs::remove_file(files.1)?;

    std::fs::rename(&path, files.0)?;

    Ok(PathBuf::from(files.0))
}

/// Divide big file into smaller. Smaller files is the slices of memory of the big one.
fn divide_files(input_file: &Path) -> Result<VecDeque<PathBuf>> {
    let mut names = VecDeque::<PathBuf>::new();
    let br = BufReader::with_capacity(BUFFER_SIZE_IN_BYTES / 2, File::open(input_file)?);
    let mut buf: Vec<f64> = Vec::with_capacity(BUFFER_SIZE_IN_BYTES / 2);
    let mut lines = br.lines();
    let mut i = 0;
    std::fs::create_dir_all("tmp")?;

    while let Some(Ok(x)) = lines.next() {
        if buf.capacity() == buf.len() {
            buf.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            unsafe {
                names.push_back(fast_file_write(i, &buf)?);
                buf.set_len(0);
            }
            i += 1;
        }
        buf.push(lexical::parse(x).unwrap());
    }
    if !buf.is_empty() {
        buf.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        unsafe {
            names.push_back(fast_file_write(i, &buf)?);
        }
    }
    Ok(names)
}

/// Get bytes of the f64. **System endianness**.
fn bytes_of_f64(val: &f64) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(val as *const f64 as *const u8, std::mem::size_of::<f64>())
    }
}

/// Unaligned bytes to f64
fn bytes_to_f64(v: Vec<u8>) -> f64 {
    //let (_, bytes_a, _) = unsafe { v.align_to::<f64>() };
    //unsafe { *(bytes_a.as_ptr() as *const f64) }
    unsafe { *(v.as_ptr() as *const f64) }
}

/// Writes all bits of slice to file
unsafe fn fast_file_write<T>(slice_number: i32, buf: &[T]) -> Result<PathBuf> {
    let ptr = buf.as_ptr() as *const u8;
    let path = PathBuf::from(format!("tmp/slice_{}", slice_number));
    let slice = std::slice::from_raw_parts(ptr, buf.len() * std::mem::size_of::<T>());
    File::create(&path)?.write_all(slice)?;
    Ok(path)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }
