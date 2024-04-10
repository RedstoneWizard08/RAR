use aes_reader::RarAesReader;
use failure::Error;
use file_block::FileBlock;
use file_writer::FileWriter;
use rar_reader::RarReader;
use std::io::prelude::*;

/// This function extracts the data from a RarReader and writes it into an file.
pub fn extract(
    file: &FileBlock,
    path: &str,
    reader: &mut RarReader,
    data_area_size: u64,
    password: Option<&str>,
) -> Result<(), Error> {
    // create file writer to create and fill the file
    let mut f_writer = FileWriter::new(file.clone(), path)?;

    // Limit the data to take from the reader
    let reader = RarReader::new(reader.take(data_area_size));

    // Initilize the decryption reader
    let mut reader = RarAesReader::new(reader, file.clone(), password);

    // loop over chunks of the data and write it to the files
    let mut data_buffer = [0u8; ::BUFFER_SIZE];
    loop {
        // read a chunk of data from the buffer
        let new_byte_count = reader.read(&mut data_buffer)?;
        let data = &mut data_buffer[..new_byte_count];

        // end loop if nothing is there anymore
        if new_byte_count == 0 {
            break;
        }

        // unpack if necessary
        // todo

        // write out the data
        if let Err(e) = f_writer.write_all(data) {
            if e.kind() == ::std::io::ErrorKind::WriteZero {
                // end loop when the file capacity is reached
                break;
            } else {
                Err(e)?;
            }
        }
    }

    // flush the data
    f_writer.flush()?;

    Ok(())
}

/// This function chains a new .rar archive file to the data stream.
/// This ensures that we can build up a big chained reader which holds the complete
/// data_area, which then can be extracted.
pub fn continue_data_next_file<'a>(
    buffer: RarReader<'a>,
    file: &mut FileBlock,
    file_name: &str,
    file_number: &mut usize,
    data_area_size: &mut u64,
) -> Result<RarReader<'a>, Error> {
    // get the next rar file name
    let mut new_file_name = file_name.to_string();
    let len = new_file_name.len();
    new_file_name.replace_range(len - 5.., &format!("{}.rar", *file_number + 1));

    // open the file
    let reader = ::std::fs::File::open(&new_file_name)?;

    // put the reader into our buffer
    let mut new_buffer = RarReader::new_from_file(reader);

    // try to parse the signature
    let version = new_buffer
        .exec_nom_parser(::sig_block::SignatureBlock::parse)
        .map_err(|_| format_err!("Can't read RAR signature"))?;
    // try to parse the archive information
    let details = new_buffer
        .exec_nom_parser(::archive_block::ArchiveBlock::parse)
        .map_err(|_| format_err!("Can't read RAR archive block"))?;
    // try to parse the file
    let new_file = new_buffer
        .exec_nom_parser(FileBlock::parse)
        .map_err(|_| format_err!("Can't read RAR file block"))?;

    // check if the next file info is the same as from prvious .rar
    if version != ::sig_block::SignatureBlock::RAR5
        || details.volume_number != *file_number as u64
        || new_file.name != file.name
    {
        return Err(format_err!(
            "The file header in the new .rar file don't match the needed file"
        ));
    }

    // Limit the data to take from the reader, when this data area
    // continues in another .rar archive file
    if new_file.head.flags.data_next {
        new_buffer = RarReader::new(new_buffer.take(new_file.head.data_area_size));
    }

    // count file number up
    *file_number += 1;

    // sum up the data area
    *data_area_size += new_file.head.data_area_size;

    // change the file with the new file
    *file = new_file;

    // chain the buffer together
    Ok(RarReader::new(buffer.chain(new_buffer)))
}
