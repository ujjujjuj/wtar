use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::{
    env, fs,
    fs::File,
    io::prelude::*,
    io::{self, Read, SeekFrom},
    path::Path,
    process, vec,
};

#[derive(Debug)]
struct WtarFile {
    path: String,
    is_dir: bool,
    size: u64,
    children: Vec<WtarFile>,
}

fn append_tree(file: &mut WtarFile) {
    let file_list = match fs::read_dir(&file.path) {
        Ok(fl) => fl,
        Err(error) => panic!("Error opening folder {}: {}", file.path, error),
    };

    for child_file in file_list {
        let child_file = match child_file {
            Ok(cf) => cf,
            Err(error) => panic!("Error opening folder: {}", error),
        };

        let md = match fs::metadata(child_file.path()) {
            Ok(md) => md,
            Err(error) => panic!(
                "Error opening {}: {}",
                child_file.path().to_string_lossy(),
                error
            ),
        };

        let mut file_obj = WtarFile {
            path: child_file.path().to_string_lossy().into_owned(),
            is_dir: md.is_dir(),
            size: md.len(),
            children: vec![],
        };

        if file_obj.is_dir {
            file_obj.size = 0;
            append_tree(&mut file_obj);
        }

        file.children.push(file_obj);
    }
}

fn serialize_tree(node: &WtarFile, meta_buf: &mut Vec<u8>, file_list: &mut Vec<String>) {
    meta_buf.extend_from_slice(&(node.path.len() as u32).to_le_bytes());
    meta_buf.extend_from_slice(node.path.as_bytes());
    meta_buf.push(node.is_dir as u8);
    if node.is_dir {
        for child_file in &node.children {
            serialize_tree(child_file, meta_buf, file_list);
        }
    } else {
        meta_buf.extend_from_slice(&(node.size).to_le_bytes());
        file_list.push(node.path.to_owned());
    }
}

fn write_files(out_file: &mut File, file_list: &Vec<String>) {
    for read_file_path in file_list {
        let mut read_file = match File::open(read_file_path) {
            Ok(f) => f,
            Err(error) => panic!("Error opening file {}: {}", read_file_path, error),
        };

        io::copy(&mut read_file, out_file).expect("Error writing to output file");
    }
}

fn create_wtar(target_folder: &str) {
    let mut target_folder = String::from(target_folder);

    if target_folder.ends_with("/") {
        target_folder.pop();
    }

    let outfile_name = format!("{}.wtar", target_folder);

    let mut root = WtarFile {
        path: target_folder.to_owned(),
        is_dir: true,
        size: 0,
        children: vec![],
    };

    append_tree(&mut root);

    let mut metadata_buf: Vec<u8> = vec![];
    let mut file_list: Vec<String> = vec![];

    serialize_tree(&root, &mut metadata_buf, &mut file_list);
    metadata_buf.splice(0..0, (metadata_buf.len() as u32).to_le_bytes());

    let mut target_file = match File::create(&outfile_name) {
        Ok(f) => f,
        Err(error) => panic!("Error creating file {}: {}", &outfile_name, error),
    };

    target_file
        .write_all(&metadata_buf)
        .expect("Failed to write to target file");

    write_files(&mut target_file, &file_list);
    drop(target_file);

    let mut target_file = File::open(&outfile_name).expect("Cannot open target file");

    let target_name_gz = format!("{}{}", &outfile_name, ".gz");
    let mut target_file_gz = match File::create(&target_name_gz) {
        Ok(f) => f,
        Err(error) => panic!("Error creating file {}: {}", target_name_gz, error),
    };
    let mut encoder = GzEncoder::new(&mut target_file_gz, Compression::best());

    io::copy(&mut target_file, &mut encoder).expect("Error writing to output file");

    drop(target_file);
    fs::remove_file(outfile_name).expect("Failed to delete intermediate file");

    encoder.finish().expect("Cannot flush target");
}

fn read_bytes_from_file(file: &mut File, n_bytes: usize) -> Vec<u8> {
    let mut tmp_buf = vec![0u8; n_bytes];
    file.read_exact(&mut tmp_buf)
        .expect("Error reading from source file");
    return tmp_buf;
}

fn read_u32_from_file(file: &mut File) -> u32 {
    return u32::from_le_bytes(read_bytes_from_file(file, 4).try_into().unwrap());
}

fn read_u64_from_file(file: &mut File) -> u64 {
    return u64::from_le_bytes(read_bytes_from_file(file, 8).try_into().unwrap());
}

fn get_overwrite_inp(file_name: &str) {
    print!(
        "A file with the name {} already exists. Overwrite? [y/n]: ",
        file_name
    );
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if input.trim() != "y" {
        println!("Exited.");
        process::exit(0);
    }
}

fn extract_wtar(infile_path: &str) {
    let infile_gz = match File::open(infile_path) {
        Ok(f) => f,
        Err(error) => panic!("Error opening source file {}: {}", infile_path, error),
    };

    let mut decoder = GzDecoder::new(infile_gz);

    let intermediate_path = Path::new(infile_path)
        .file_stem()
        .expect("Invalid file name");
    let mut infile = match File::create(intermediate_path) {
        Ok(f) => f,
        Err(error) => panic!("Error creating intermediate file: {}", error),
    };

    io::copy(&mut decoder, &mut infile).expect("Error decompressing file");

    drop(infile);
    let mut infile = match File::open(intermediate_path) {
        Ok(f) => f,
        Err(error) => panic!("Error opening intermediate file: {}", error),
    };

    let data_offset = read_u32_from_file(&mut infile) as u64 + 4;

    let mut tmp_data_offset = data_offset;

    let mut overwrite_pref = false;

    let mut curr_offset = 0;
    while curr_offset < data_offset {
        let name_len = read_u32_from_file(&mut infile) as usize;
        let name_buf = read_bytes_from_file(&mut infile, name_len);
        let file_path = String::from_utf8_lossy(&name_buf).to_string();
        let is_dir = read_bytes_from_file(&mut infile, 1)[0] != 0;

        if Path::new(&file_path).exists() {
            if !overwrite_pref {
                get_overwrite_inp(&file_path);
                overwrite_pref = true;
            }

            if is_dir {
                fs::remove_dir_all(&file_path).expect("Failed to delete existing folder");
            } else {
                fs::remove_file(&file_path).expect("Failed to delete file");
            }
        }
        if is_dir {
            match fs::create_dir(&file_path) {
                Ok(()) => (),
                Err(error) => panic!("Cannot create folder {}: {}", file_path, error),
            };
        } else {
            let file_len = read_u64_from_file(&mut infile);
            let mut outfile = match File::create(&file_path) {
                Ok(f) => f,
                Err(error) => panic!("Error creating file {}: {}", file_path, error),
            };

            curr_offset = infile.seek(SeekFrom::Current(0)).unwrap();
            infile
                .seek(SeekFrom::Start(tmp_data_offset))
                .expect("Failed to seek");
            io::copy(
                &mut io::Read::by_ref(&mut infile).take(file_len),
                &mut outfile,
            )
            .expect("Error copying from file");
            tmp_data_offset += file_len;
            infile
                .seek(SeekFrom::Start(curr_offset))
                .expect("Failed to seek");
        }
        curr_offset = infile.seek(SeekFrom::Current(0)).unwrap();
    }

    fs::remove_file(intermediate_path).expect("Failed to delete intermediate file");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 3 && args[1] == "-c" {
        create_wtar(&args[2]);
    } else if args.len() == 3 && args[1] == "-e" {
        extract_wtar(&args[2]);
    } else {
        println!("Usage: wtar <OPTION> <FILE/FOLDER>...\n\nExamples:\n  wtar -c infolder/            Create an archive named infolder.wtar.gz from the folder infolder\n  wtar -e archive.wtar.gz      Extract the archive archive.wtar");
    }
}
