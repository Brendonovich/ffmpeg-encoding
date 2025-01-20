use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SizedSample,
};
use ffmpeg_next::{self as ffmpeg, codec, decoder, frame};

const path: &str = r#"/Users/brendonovich/Library/Application Support/so.cap.desktop.dev/recordings/3b61ac67-e507-48c0-a029-8200ca6e21e2.cap/content/segments/segment-1/audio-input.mp3"#;

fn main() -> anyhow::Result<()> {
    let mut input_ctx = ffmpeg::format::input(&path).unwrap();
    let input_stream = input_ctx
        .streams()
        .best(ffmpeg::media::Type::Audio)
        .unwrap();

    let decoder_ctx = codec::context::Context::from_parameters(input_stream.parameters()).unwrap();
    let mut decoder = decoder_ctx.decoder().audio().unwrap();
    decoder.set_parameters(input_stream.parameters()).unwrap();
    decoder.set_packet_time_base(input_stream.time_base());

    dbg!(decoder.rate());
    dbg!(decoder.format());

    let mut samples: Vec<f32> = vec![];

    let mut i = 0;

    let mut decode_packets = |decoder: &mut decoder::Audio| {
        let mut frame = frame::Audio::empty();
        while decoder.receive_frame(&mut frame).is_ok() {
            i += 1;
            samples.extend(
                frame
                    .data(0)
                    .chunks_exact(4)
                    .map(|bytes| f32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
            );
            frame = frame::Audio::empty();

            // Do something with the frame
        }
    };

    let index = input_stream.index();
    for (stream, mut packet) in input_ctx.packets() {
        if stream.index() == index {
            decoder.send_packet(&packet).unwrap();
            decode_packets(&mut decoder);
        }
    }

    decoder.send_eof().unwrap();
    decode_packets(&mut decoder);

    dbg!(i);
    dbg!(samples.len());

    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .expect("failed to find output device");
    println!("Output device: {}", device.name()?);

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    match config.sample_format() {
        cpal::SampleFormat::I8 => run::<i8>(&device, &config.into(), samples),
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), samples),
        // cpal::SampleFormat::I24 => run::<I24>(&device, &config.into()),
        cpal::SampleFormat::I32 => run::<i32>(&device, &config.into(), samples),
        // cpal::SampleFormat::I48 => run::<I48>(&device, &config.into()),
        cpal::SampleFormat::I64 => run::<i64>(&device, &config.into(), samples),
        cpal::SampleFormat::U8 => run::<u8>(&device, &config.into(), samples),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), samples),
        // cpal::SampleFormat::U24 => run::<U24>(&device, &config.into()),
        cpal::SampleFormat::U32 => run::<u32>(&device, &config.into(), samples),
        // cpal::SampleFormat::U48 => run::<U48>(&device, &config.into()),
        cpal::SampleFormat::U64 => run::<u64>(&device, &config.into(), samples),
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), samples),
        cpal::SampleFormat::F64 => run::<f64>(&device, &config.into(), samples),
        sample_format => panic!("Unsupported sample format '{sample_format}'"),
    }
}

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    bytes: Vec<f32>,
) -> Result<(), anyhow::Error>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    // Produce a sinusoid of maximum amplitude.
    let mut i = 0;
    dbg!(bytes.len());
    let mut next_value = move || {
        if i >= bytes.len() {
            return 0.0;
        }
        let value = bytes[i];
        i += 1;
        // dbg!(i);
        value
        // sample_clock = (sample_clock + 1.0) % sample_rate;
        // (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
    };

    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value: T = T::from_sample(next_value());
                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(10000));

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: Sample + FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = T::from_sample(next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
