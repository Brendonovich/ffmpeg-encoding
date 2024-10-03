/*
 * Trying to replicate the following command:
 * ffmpeg -f f32le -ar 44100 -ac 1 -thread_queue_size 4096 -i audio.raw -f mp3 -b:a 128k -ar 44100 -ac 1
 */

use std::{
    ffi::CStr,
    ptr::{null, null_mut},
};

use ffmpeg::ffi::*;
use ffmpeg_next as ffmpeg;

fn main() {
    unsafe {
        let mut output = null_mut();
        let filename = c"output.mp3".as_ptr();
        avformat_alloc_output_context2(&mut output, null(), null(), filename);

        let codec = avcodec_find_encoder(AVCodecID::AV_CODEC_ID_MP3);

        let stream = avformat_new_stream(output, null());
        (*stream).id = ((*output).nb_streams - 1) as i32;

        let mut encoder = avcodec_alloc_context3(codec);

        (*encoder).bit_rate = 128000;
        (*encoder).sample_rate = 44100;
        (*encoder).sample_fmt = AVSampleFormat::AV_SAMPLE_FMT_S16P;

        if !is_valid_sample_fmt(codec, (*encoder).sample_fmt) {
            panic!(
                "Encoder does not support sample format {:?}",
                CStr::from_ptr(av_get_sample_fmt_name((*encoder).sample_fmt)),
            )
        }

        select_channel_layout(codec, &mut (*encoder).ch_layout);

        avcodec_open2(encoder, codec, null_mut());

        let mut frame = av_frame_alloc();
        (*frame).nb_samples = (*encoder).frame_size;
        (*frame).format = (*encoder).sample_fmt as i32;
        av_channel_layout_copy(&mut (*frame).ch_layout, &(*encoder).ch_layout);
        av_frame_get_buffer(frame, 0);

        let mut packet = av_packet_alloc();

        let mut t = 0.0;
        let tincr = 2.0 * M_PI * 440.0 / (*encoder).sample_rate as f64;

        for i in 0..200 {
            println!("i: {i}");
            av_frame_make_writable(frame);

            let samples = (*frame).data[0] as *mut u16;

            for j in 0..(*encoder).frame_size {
                println!("j before: {j}");
                *samples.offset(2 * j as isize) = (sin(t as f64) * 10_000.0) as u16;
                println!("j after: {j}");

                t += tincr;
            }

            (*frame).pts = (*encoder).frame_size as i64 * i as i64;

            println!("encoding {i}");

            encode(output, encoder, stream, frame, packet);
        }

        encode(output, encoder, stream, null(), packet);

        if !((*(*output).oformat).flags & AVFMT_NOFILE == 1) {
            avio_closep(&mut (*output).pb);
        }

        avcodec_free_context(&mut encoder);
        av_frame_free(&mut frame);
        av_packet_free(&mut packet);

        avformat_free_context(output);
    }
}

unsafe fn is_valid_sample_fmt(codec: *const AVCodec, sample_fmt: AVSampleFormat) -> bool {
    let mut ptr = (*codec).sample_fmts;

    while *ptr != AVSampleFormat::AV_SAMPLE_FMT_NONE {
        if *ptr == sample_fmt {
            return true;
        }
        ptr = ptr.add(1);
    }

    false
}

unsafe fn encode(
    output: *mut AVFormatContext,
    encoder: *mut AVCodecContext,
    stream: *const AVStream,
    frame: *const AVFrame,
    packet: *mut AVPacket,
) {
    dbg!(output, encoder, stream, frame, packet);
    avcodec_send_frame(encoder, frame);

    receive_and_write_packets(output, encoder, stream, packet);
}

unsafe fn receive_and_write_packets(
    output: *mut AVFormatContext,
    encoder: *mut AVCodecContext,
    stream: *const AVStream,
    packet: *mut AVPacket,
) {
    dbg!(output, encoder, stream, packet);

    while avcodec_receive_packet(encoder, packet) >= 0 {
        (*packet).stream_index = (*stream).index;

        av_interleaved_write_frame(output, packet);
        av_packet_unref(packet);
    }
}

unsafe fn select_channel_layout(codec: *const AVCodec, dest: *mut AVChannelLayout) -> i32 {
    // if (*codec).ch_layouts.is_null() {
    return av_channel_layout_copy(
        dest,
        &AVChannelLayout {
            order: AVChannelOrder::AV_CHANNEL_ORDER_NATIVE,
            nb_channels: 1,
            u: AVChannelLayout__bindgen_ty_1 {
                mask: AV_CH_LAYOUT_MONO,
            },
            opaque: null_mut(),
        },
    );
    // }

    // let mut best_nb_channels = 0;
    // let mut best_ch_layout = null();
    // let mut p = (*codec).ch_layouts;

    // while !p.is_null() {
    //     let nb_channels = (*p).nb_channels;

    //     dbg!(dest, nb_channels, best_nb_channels);

    //     if nb_channels > best_nb_channels {
    //         best_ch_layout = p;
    //         best_nb_channels = nb_channels;
    //     }

    //     p = p.add(1);
    // }

    // av_channel_layout_copy(dest, best_ch_layout)
}
