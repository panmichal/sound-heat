use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub fn decode(file: std::fs::File) -> Result<Vec<f32>, Error> {
    let mut samples: Vec<f32> = Vec::new();
    // Create a MediaSourceStream from the file.
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry guess what format reader is appropriate.
    let mut hint = Hint::new();
    hint.with_extension("mp3");

    // Use the default format registry to probe the media source stream for a format.
    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    // Get the instantiated format reader.
    let mut format = probed.format;

    // Find the first audio track with a known codec.
    let track = format.default_track().unwrap();
    let track_id = track.id;

    // Create a decoder for the track.
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let mut sample_count = 0;
    let mut sample_buf = None;

    // Decode packets until there are no more packets left.
    loop {
        // Get the next packet from the format reader.
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(_)) => break, // End of stream
            Err(e) => return Err(e),
        };

        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                // Get the audio buffer as a slice of i16 samples.

                if sample_buf.is_none() {
                    let spec = *audio_buf.spec();
                    let duration = audio_buf.capacity() as u64;
                    sample_buf = Some(SampleBuffer::<f32>::new(duration, spec));
                }

                if let Some(buf) = &mut sample_buf {
                    buf.copy_interleaved_ref(audio_buf);
                    sample_count += buf.samples().len();

                    print!("\rDecoded {} samples", sample_count);

                    samples.extend_from_slice(buf.samples());
                }
            }
            Err(Error::DecodeError(_)) => {
                // Ignore decode errors for individual packets.
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Ok(samples)
}
