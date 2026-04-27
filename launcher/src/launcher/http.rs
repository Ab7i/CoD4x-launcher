use curl::easy::{Easy2, Handler, List, WriteError};
use std::io::Write;
use std::time::Duration;

pub trait Progress {
    fn progress(&self, _dltotal: f64, _dlnow: f64) -> bool {
        true
    }
}

pub struct ProgressCallback {
    callback: Box<dyn Fn(f64) -> bool + 'static>,
}

impl ProgressCallback {
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(f64) -> bool + 'static,
    {
        Self {
            callback: Box::new(callback),
        }
    }
}

impl Progress for ProgressCallback {
    fn progress(&self, dltotal: f64, dlnow: f64) -> bool {
        let p = if dltotal > 0.0 {
            dlnow / dltotal * 100.0
        } else {
            0.0
        };

        self.callback.as_ref()(p)
    }
}

pub struct DummyProgress;
impl Progress for DummyProgress {}

struct FileCollector<'a, P> {
    file: std::fs::File,
    progress: &'a P,
}

impl<'a, P: Progress> FileCollector<'a, P> {
    pub fn new(file: std::fs::File, progress: &'a P) -> Self {
        Self { file, progress }
    }
}

impl<'a, P: Progress> Handler for FileCollector<'a, P> {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        if self.file.write_all(data).is_err() {
            Ok(0)
        } else {
            Ok(data.len())
        }
    }

    fn progress(&mut self, dltotal: f64, dlnow: f64, _ultotal: f64, _ulnow: f64) -> bool {
        self.progress.progress(dltotal, dlnow)
    }
}

pub fn download_file<P: Progress>(
    url: &str,
    path: &std::path::Path,
    progress: &P,
) -> anyhow::Result<()> {
    let easy = build_easy_get(
        url,
        FileCollector::new(std::fs::File::create(path)?, progress),
    )?;
    easy.perform()?;
    Ok(())
}

pub struct Collector {
    data: Vec<u8>,
}

impl Collector {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl Handler for Collector {
    fn write(&mut self, data: &[u8]) -> Result<usize, WriteError> {
        self.data.extend_from_slice(data);
        Ok(data.len())
    }
}

pub struct RequestBuilder<'a> {
    url: &'a str,
    timeout: Option<Duration>,
    headers: List,
}

impl<'a> RequestBuilder<'a> {
    pub fn new(url: &'a str) -> Self {
        Self {
            url,
            timeout: None,
            headers: List::new(),
        }
    }

    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn add_header(&mut self, header: &str) -> Result<&mut Self, curl::Error> {
        self.headers.append(header)?;
        Ok(self)
    }

    pub fn build(self) -> Result<Easy2<Collector>, curl::Error> {
        let mut easy = build_easy_get(self.url, Collector::new())?;
        if let Some(timeout) = self.timeout {
            easy.timeout(timeout)?;
        }
        easy.http_headers(self.headers)?;
        Ok(easy)
    }
}

fn build_easy_get<H: Handler>(url: &str, handler: H) -> Result<Easy2<H>, curl::Error> {
    let mut easy = Easy2::new(handler);
    easy.get(true)?;
    easy.follow_location(true)?;
    easy.url(url)?;
    // TODO: consider using a user agent designated for this cod4 launcher
    easy.useragent("curl/8.9.1")?;
    easy.progress(true)?;
    Ok(easy)
}
