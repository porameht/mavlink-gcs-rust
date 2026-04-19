use std::sync::{Arc, Mutex};

use anyhow::Result;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;

/// Video frame data — extracted from GStreamer pipeline
#[derive(Clone)]
pub struct VideoFrame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>, // RGB bytes
}

/// Start GStreamer pipeline to receive RTP H.264 on UDP port
pub fn start_video_receiver(port: u16, frame_buf: Arc<Mutex<Option<VideoFrame>>>) -> Result<gstreamer::Pipeline> {
    gstreamer::init()?;

    let pipeline_str = format!(
        "udpsrc port={port} caps=\"application/x-rtp,media=video,encoding-name=H264,payload=96\" \
         ! rtph264depay \
         ! avdec_h264 \
         ! videoconvert \
         ! video/x-raw,format=RGB \
         ! appsink name=sink emit-signals=true sync=false"
    );

    let pipeline = gstreamer::parse::launch(&pipeline_str)?
        .downcast::<gstreamer::Pipeline>()
        .map_err(|_| anyhow::anyhow!("Failed to create pipeline"))?;

    let appsink = pipeline
        .by_name("sink")
        .ok_or_else(|| anyhow::anyhow!("No appsink found"))?
        .downcast::<AppSink>()
        .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Error)?;
                let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                let caps = sample.caps().ok_or(gstreamer::FlowError::Error)?;
                let info = gstreamer_video::VideoInfo::from_caps(caps)
                    .map_err(|_| gstreamer::FlowError::Error)?;

                let map = buffer.map_readable().map_err(|_| gstreamer::FlowError::Error)?;

                let frame = VideoFrame {
                    width: info.width(),
                    height: info.height(),
                    data: map.as_slice().to_vec(),
                };

                if let Ok(mut buf) = frame_buf.lock() {
                    *buf = Some(frame);
                }

                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );

    pipeline.set_state(gstreamer::State::Playing)?;

    Ok(pipeline)
}

/// Start a test video source (no real camera needed)
pub fn start_test_video(frame_buf: Arc<Mutex<Option<VideoFrame>>>) -> Result<gstreamer::Pipeline> {
    gstreamer::init()?;

    let pipeline_str =
        "videotestsrc pattern=ball is-live=true \
         ! video/x-raw,width=640,height=480,framerate=30/1 \
         ! videoconvert \
         ! video/x-raw,format=RGB \
         ! appsink name=sink emit-signals=true sync=false";

    let pipeline = gstreamer::parse::launch(pipeline_str)?
        .downcast::<gstreamer::Pipeline>()
        .map_err(|_| anyhow::anyhow!("Failed to create pipeline"))?;

    let appsink = pipeline
        .by_name("sink")
        .ok_or_else(|| anyhow::anyhow!("No appsink found"))?
        .downcast::<AppSink>()
        .map_err(|_| anyhow::anyhow!("Failed to downcast to AppSink"))?;

    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Error)?;
                let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
                let caps = sample.caps().ok_or(gstreamer::FlowError::Error)?;
                let info = gstreamer_video::VideoInfo::from_caps(caps)
                    .map_err(|_| gstreamer::FlowError::Error)?;

                let map = buffer.map_readable().map_err(|_| gstreamer::FlowError::Error)?;

                let frame = VideoFrame {
                    width: info.width(),
                    height: info.height(),
                    data: map.as_slice().to_vec(),
                };

                if let Ok(mut buf) = frame_buf.lock() {
                    *buf = Some(frame);
                }

                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );

    pipeline.set_state(gstreamer::State::Playing)?;

    Ok(pipeline)
}
