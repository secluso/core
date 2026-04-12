use std::fs::{self, File};
use std::io::{self, Read, Write, BufRead, BufReader, BufWriter};
use std::time::Instant;
use std::path::Path;
use log::{debug, info, error};
use openmls::prelude::QueuedProposal;
use crate::mls_client::MlsClient;
use crate::video_net_info::{VideoNetInfo, VIDEONETINFO_SANITY};
use crate::thumbnail_meta_info::{ThumbnailMetaInfo, THUMBNAIL_SANITY};

pub fn decrypt_video_file(
    motion_mls_client: &mut MlsClient,
    enc_pathname: &str,
) -> io::Result<String> {
    let total_start = Instant::now();
    let file_dir = motion_mls_client.get_file_dir();
    info!("File dir: {}", file_dir);
    let mut enc_file = File::open(enc_pathname).expect("Could not open encrypted file");

    // The first message is a vec of update proposals (which could be empty)
    let msg = read_next_msg_from_file(&mut enc_file)?;
    let proposals_start = Instant::now();
    let update_proposals: Vec<QueuedProposal> = bincode::deserialize(&msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    motion_mls_client.store_update_proposals(update_proposals)?;
    let proposals_ms = proposals_start.elapsed().as_millis();

    let msg = read_next_msg_from_file(&mut enc_file)?;
    // The second message is the commit message
    let commit_start = Instant::now();
    motion_mls_client.decrypt(msg, false)?;
    let commit_ms = commit_start.elapsed().as_millis();

    let enc_msg = read_next_msg_from_file(&mut enc_file)?;
    // The third message is the video info
    let info_start = Instant::now();
    let dec_msg = motion_mls_client.decrypt(enc_msg, true)?;
    let info_ms = info_start.elapsed().as_millis();

    let info: VideoNetInfo = bincode::deserialize(&dec_msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    if info.sanity != *VIDEONETINFO_SANITY || info.num_msg == 0 {
        return Err(io::Error::other("Error: Corrupt VideoNetInfo message."));
    }

    #[cfg(test)]
    {
        if std::env::var("DECRYPT_VIDEO_FILE_CRASH").is_ok() {
            return Err(io::Error::other("Error: A test crash occurred."));
        }
    }

    // The rest of the messages are video data
    //Note: we're building the filename based on the timestamp in the message.
    //The encrypted filename however is not protected and hence the server could have changed it.
    //Therefore, it is possible that the names won't match.
    //This is not an issue.
    //We should use the timestamp in the decrypted filename going forward
    //and discard the encrypted filename.
    let dec_filename = format!("video_{}.mp4", info.timestamp);
    let dec_pathname: String = format!("{}/videos/{}", file_dir, dec_filename);

    if Path::new(&dec_pathname).exists() {
        debug!(
            "decrypt_video timings (duplicate): commit={}ms info={}ms total={}ms",
            commit_ms,
            info_ms,
            total_start.elapsed().as_millis()
        );
        return Ok(dec_filename);
    }

    info!("Decrypted pathname: {}", dec_pathname);

    let mut dec_file = File::create(&dec_pathname).expect("Could not create decrypted file");

    let chunk_start = Instant::now();
    for expected_chunk_number in 0..info.num_msg {
        let enc_msg = read_next_msg_from_file(&mut enc_file)?;
        let dec_msg = motion_mls_client.decrypt(enc_msg, true)?;

        // check the chunk number
        if dec_msg.len() < 8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Error: too few bytes!".to_string(),
            ));
        }

        let chunk_number = u64::from_be_bytes(dec_msg[..8].try_into().unwrap());
        if chunk_number != expected_chunk_number {
            let _ = fs::remove_file(&dec_pathname);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Error: invalid chunk number!".to_string(),
            ));
        }

        let _ = dec_file.write_all(&dec_msg[8..]);
    }
    let chunk_ms = chunk_start.elapsed().as_millis();

    // Here, we first make sure the dec_file is flushed.
    // Then, we save groups state, which persists the update.
    let flush_start = Instant::now();
    dec_file.flush().unwrap();
    dec_file.sync_all().unwrap();
    motion_mls_client.save_group_state().unwrap();
    let flush_ms = flush_start.elapsed().as_millis();

    debug!(
        "decrypt_video timings: proposals={}ms commit={}ms info={}ms chunks={}ms flush={}ms total={}ms (chunks={})",
        proposals_ms,
        commit_ms,
        info_ms,
        chunk_ms,
        flush_ms,
        total_start.elapsed().as_millis(),
        info.num_msg
    );

    Ok(dec_filename)
}

pub fn decrypt_thumbnail_file(
    thumbnail_mls_client: &mut MlsClient,
    enc_pathname: &str,
    pending_meta_directory: &str,
) -> io::Result<String> {
    let total_start = Instant::now();
    let file_dir = thumbnail_mls_client.get_file_dir();
    info!("File dir: {}", file_dir);

    let mut enc_file = File::open(enc_pathname).expect("Could not open encrypted file");

    // The first message is a vec of update proposals (which could be empty)
    let msg = read_next_msg_from_file(&mut enc_file)?;
    let proposals_start = Instant::now();
    let update_proposals: Vec<QueuedProposal> = bincode::deserialize(&msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    thumbnail_mls_client.store_update_proposals(update_proposals)?;
    let proposals_ms = proposals_start.elapsed().as_millis();

    let msg = read_next_msg_from_file(&mut enc_file)?;
    // The second message is a commit message
    let commit_start = Instant::now();
    thumbnail_mls_client.decrypt(msg, false)?;
    let commit_ms = commit_start.elapsed().as_millis();

    let enc_msg = read_next_msg_from_file(&mut enc_file)?;
    // The third message is the timestamp
    let meta_start = Instant::now();
    let dec_msg = thumbnail_mls_client.decrypt(enc_msg, true)?;
    let meta_ms = meta_start.elapsed().as_millis();

    let thumbnail_meta_info: ThumbnailMetaInfo = bincode::deserialize(&dec_msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    if thumbnail_meta_info.sanity != *THUMBNAIL_SANITY {
        return Err(io::Error::other("Error: Corrupt ThumbalMetaInfo message."));
    }

    #[cfg(test)]
    {
        if std::env::var("DECRYPT_THUMBNAIL_FILE_CRASH").is_ok() {
            return Err(io::Error::other("Error: A test crash occurred."));
        }
    }

    // Do not trust the sender-provided filename here.
    // The timestamp is the stable identifier for thumbnails, and deriving the path from it prevents path traversal through attacker-crafted metadata.
    let dec_filename = ThumbnailMetaInfo::get_filename_from_timestamp(thumbnail_meta_info.timestamp);
    let dec_pathname: String = format!("{}/videos/{}", file_dir, dec_filename);

    if Path::new(&dec_pathname).exists() {
        // TODO: Should this be an error?
        debug!(
            "decrypt_thumbnail timings (duplicate): proposals={}ms commit={}ms meta={}ms total={}ms",
            proposals_ms,
            commit_ms,
            meta_ms,
            total_start.elapsed().as_millis()
        );
        return Ok(dec_filename);
    }

    // Write a metadata file for the thumbnail, which will be deleted later and stored in the database via the pending processor.
    let dec_meta_file_path: String = format!(
        "{}/meta_{}.txt",
        pending_meta_directory, thumbnail_meta_info.timestamp
    );

    let meta_file = File::create(&dec_meta_file_path)?;
    let mut meta_file_writer = BufWriter::new(meta_file);

    // Write JSON data to file.
    serde_json::to_writer(&mut meta_file_writer, &thumbnail_meta_info.detections)
        .map_err(std::io::Error::other)?;

    let mut dec_file = File::create(&dec_pathname).expect("Could not create decrypted file");

    let enc_msg = read_next_msg_from_file(&mut enc_file)?;
    let payload_start = Instant::now();
    let dec_msg = thumbnail_mls_client.decrypt(enc_msg, true)?;
    let payload_ms = payload_start.elapsed().as_millis();

    let _ = dec_file.write_all(&dec_msg);

    // Here, we first make sure the dec_file is flushed.
    // Then, we save groups state, which persists the update.
    let flush_start = Instant::now();
    dec_file.flush().unwrap();
    dec_file.sync_all().unwrap();
    thumbnail_mls_client.save_group_state().unwrap();
    let flush_ms = flush_start.elapsed().as_millis();

    debug!(
        "decrypt_thumbnail timings: commit={}ms meta={}ms payload={}ms flush={}ms total={}ms (bytes={})",
        commit_ms,
        meta_ms,
        payload_ms,
        flush_ms,
        total_start.elapsed().as_millis(),
        dec_msg.len()
    );

    Ok(dec_filename)
}

fn read_next_msg_from_file(file: &mut File) -> io::Result<Vec<u8>> {
    let mut len_buffer = [0u8; 4];
    let len_bytes_read = file.read(&mut len_buffer)?;
    if len_bytes_read != 4 {
        return Err(io::Error::other(
            "Error: not enough bytes to read the len from file".to_string(),
        ));
    }

    let msg_len = u32::from_be_bytes(len_buffer);

    let mut buffer = vec![0; msg_len.try_into().unwrap()];
    let bytes_read = file.read(&mut buffer)?;
    if bytes_read != msg_len as usize {
        return Err(io::Error::other(
            "Error: not enough bytes to read the message from file".to_string(),
        ));
    }

    Ok(buffer)
}

pub fn encrypt_video_file(
    motion_mls_client: &mut MlsClient,
    video_pathname: &str,
    enc_pathname: &str,
    timestamp: u64,
) -> io::Result<u64> {
    debug!("Starting to encrypt video.");
    let mut enc_file =
        File::create(&enc_pathname).expect("Could not create encrypted video file");

    let update_proposals = motion_mls_client.get_update_proposals()?;
    let update_proposals_msg = bincode::serialize(&update_proposals).unwrap();
    append_to_file(&enc_file, update_proposals_msg);

    let (commit_msg, epoch) = motion_mls_client.update()?;

    append_to_file(&enc_file, commit_msg);

    let file = File::open(video_pathname).expect("Could not open video file to send");
    let file_len = file.metadata().unwrap().len();

    // FIXME: why this chunk size? Test larger and smaller chunks.
    const READ_SIZE: usize = 64 * 1024;
    let mut reader = BufReader::with_capacity(READ_SIZE, file);

    let net_info = VideoNetInfo::new(timestamp, file_len, READ_SIZE as u64);

    let msg = motion_mls_client
        .encrypt(&bincode::serialize(&net_info).unwrap())
        .inspect_err(|_| {
            error!("encrypt() returned error:");
        })?;
    append_to_file(&enc_file, msg);

    for chunk_number in 0..net_info.num_msg {
        // We include the chunk number in the chunk itself (and check it in the app)
        // to prevent a malicious server from reordering the chunks.
        let mut buffer: Vec<u8> = chunk_number.to_be_bytes().to_vec();
        buffer.extend(reader.fill_buf().unwrap());
        let length = buffer.len();
        // Sanity checks
        if chunk_number < (net_info.num_msg - 1) {
            assert_eq!(length, READ_SIZE + 8);
        } else {
            assert_eq!(
                length,
                (<u64 as TryInto<usize>>::try_into(file_len).unwrap() % READ_SIZE) + 8
            );
        }

        let msg = motion_mls_client.encrypt(&buffer).inspect_err(|_| {
            error!("encrypt() returned error:");
        })?;
        append_to_file(&enc_file, msg);
        reader.consume(length);
    }

    // Here, we first make sure the enc_file is flushed.
    // Then, we save groups state, which persists the update.
    // When we return from this function, we enqueue to be uploaded to the server.
    enc_file.flush().unwrap();
    enc_file.sync_all().unwrap();
    motion_mls_client.save_group_state().unwrap();

    Ok(epoch)
}

pub fn encrypt_thumbnail_file(
    thumbnail_mls_client: &mut MlsClient,
    thumbnail_pathname: &str,
    enc_pathname: &str,
    thumbnail_info: &mut ThumbnailMetaInfo,
) -> io::Result<u64> {
    debug!("Starting to encrypt thumbnail.");
    let mut enc_file =
        File::create(&enc_pathname).expect("Could not create encrypted video file");

    let update_proposals = thumbnail_mls_client.get_update_proposals()?;
    let update_proposals_msg = bincode::serialize(&update_proposals).unwrap();
    append_to_file(&enc_file, update_proposals_msg);

    // Update MLS epoch
    let (commit_msg, thumbnail_epoch) = thumbnail_mls_client.update()?;

    append_to_file(&enc_file, commit_msg);

    // We need to store the timestamp to match against the video's, as otherwise we only have epoch-level info (which can vary between videos and timestamps easily)
    let msg = thumbnail_mls_client
        .encrypt(&bincode::serialize(&thumbnail_info).unwrap())
        .inspect_err(|_| {
            error!("encrypt() returned error:");
        })?;
    append_to_file(&enc_file, msg);

    let mut file = File::open(thumbnail_pathname).expect("Could not open video file to send");
    let mut thumbnail_data: Vec<u8> = Vec::new();
    file.read_to_end(&mut thumbnail_data)?;

    let msg = thumbnail_mls_client.encrypt(&thumbnail_data).inspect_err(|_| {
        error!("encrypt() returned error:");
    })?;
    append_to_file(&enc_file, msg);

    // Here, we first make sure the enc_file is flushed.
    // Then, we save groups state, which persists the update.
    // Then, we enqueue to be uploaded to the server.
    enc_file.flush().unwrap();
    enc_file.sync_all().unwrap();
    thumbnail_mls_client.save_group_state().unwrap();

    Ok(thumbnail_epoch)
}

fn append_to_file(mut file: &File, msg: Vec<u8>) {
    let msg_len: u32 = msg.len().try_into().unwrap();
    let msg_len_data = msg_len.to_be_bytes();
    let _ = file.write_all(&msg_len_data);
    let _ = file.write_all(&msg);
}