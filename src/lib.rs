//! A very simple crate to get images from your web cam.
//!
//! To decouple for example the rendering from the gathering of images, the image
//! capturing is done in another thread.

#![warn(missing_docs)]

#[macro_use]
extern crate error_chain;
extern crate rscam;

pub mod errors;

use std::sync::mpsc;
use std::thread;

use errors::*;

/// The format of the image, which also determines the size of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// In `YUYV`, the colors are in the following order:
    ///
    /// `Luminance0, U01, Luminance1, V01, ...`
    ///
    /// This means, that for every two pixels there is only one color information (the ratios
    /// U and V). But every pixel has luminance information.
    YUYV,
    /// Three byte `RGB`-format.
    RGB,
    /// Compressed MJPG format, can be used to save `jpg`-images directly.
    MJPG,
}

/// The configuration of the camera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// The resolution (width, height), e.g. (640, 480). The default resolution is said value.
    pub resolution: (u32, u32),
    /// The frames per second, common values include 30, 25, 24 or 15. The default frame rate is 30 fps.
    pub frames_per_second: u32,
    /// The image format the camera should produce. The default one is YUYV.
    pub image_format: ImageFormat,
}

/// Default implementation for Config.
impl Default for Config {
    fn default() -> Config {
        Config {
            resolution: (640, 480),
            frames_per_second: 30,
            image_format: ImageFormat::YUYV,
        }
    }
}

/// This struct is the main entry point into this library.
pub struct CameraThread {
    join_handle: thread::JoinHandle<Result<()>>,
    image_receiver: mpsc::Receiver<rscam::Frame>,
}

fn initialize_camera(camera_path: &str) -> Result<rscam::Camera> {
    use rscam::Camera;

    Camera::new(camera_path)
        .chain_err(|| ErrorKind::CameraNotAvailable(camera_path.to_string()))
}

impl CameraThread {
    /// Creates a new camera thread. The camera is turned on directly.
    pub fn new<T: AsRef<str>>(camera_path: T, configuration: Config) -> Result<CameraThread> {
        let (image_sender, image_receiver) = mpsc::sync_channel::<rscam::Frame>(1);
        let mut camera = initialize_camera(camera_path.as_ref())?;

        camera.start(&rscam::Config {
            interval: (1, configuration.frames_per_second),
            resolution: configuration.resolution,
            format: match configuration.image_format {
                ImageFormat::YUYV => b"YUYV",
                ImageFormat::RGB => b"RGB3",
                ImageFormat::MJPG => b"MJPG",
            },
            ..Default::default()
        }).chain_err(|| ErrorKind::ConfigurationError)?;

        let join_handle = thread::spawn(move || {
            'camera: loop {
                let frame = camera.capture()
                    .chain_err(|| ErrorKind::CaptureError)?;

                // currently we wait until the other half processed the previous image
                image_sender.send(frame)
                    .chain_err(|| ErrorKind::SendError)?;
            }
        });

        Ok(CameraThread {
            join_handle: join_handle,
            image_receiver: image_receiver,
        })
    }

    /// Return the next frame, if available.
    pub fn next_frame(&mut self) -> Result<Option<Vec<u8>>> {
        let possible_image = self.image_receiver.try_recv();
        match possible_image {
            Ok(image) => Ok(Some(image[..].to_vec())),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(ErrorKind::ReceiveError.into())
        }
    }
}
