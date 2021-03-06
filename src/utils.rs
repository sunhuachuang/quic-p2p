// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::ctx_mut;
use crate::dirs::Dirs;
use crate::error::Error;
use crate::event::Event;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::net::SocketAddr;
use std::path::Path;

/// Result used by `QuicP2p`.
pub type R<T> = Result<T, Error>;

/// This is to terminate the connection attempt should it take too long to mature to completeness.
pub type ConnectTerminator = tokio::sync::mpsc::Sender<()>;
/// Obtain a `ConnectTerminator` paired with a corresponding receiver.
pub fn connect_terminator() -> (ConnectTerminator, tokio::sync::mpsc::Receiver<()>) {
    tokio::sync::mpsc::channel(1)
}

/// Get the project directory
#[cfg(any(
    all(
        unix,
        not(any(target_os = "android", target_os = "androideabi", target_os = "ios"))
    ),
    windows
))]
#[inline]
pub fn project_dir() -> R<Dirs> {
    let dirs = directories::ProjectDirs::from("net", "MaidSafe", "quic-p2p")
        .ok_or_else(|| Error::Io(::std::io::ErrorKind::NotFound.into()))?;
    Ok(Dirs::Desktop(dirs))
}

/// Get the project directory
#[cfg(not(any(
    all(
        unix,
        not(any(target_os = "android", target_os = "androideabi", target_os = "ios"))
    ),
    windows
)))]
#[inline]
pub fn project_dir() -> R<Dirs> {
    Err(Error::Configuration(
        "No default project dir on non-desktop platforms. User must provide an override path."
            .to_string(),
    ))
}

/// Convert binary data to a diplay-able format
#[inline]
pub fn bin_data_format(data: &[u8]) -> String {
    let len = data.len();
    if len < 8 {
        return format!("[ {:?} ]", data);
    }

    format!(
        "[ {:02x} {:02x} {:02x} {:02x}..{:02x} {:02x} {:02x} {:02x} ]",
        data[0],
        data[1],
        data[2],
        data[3],
        data[len - 4],
        data[len - 3],
        data[len - 2],
        data[len - 1]
    )
}

/// Handle error in communication.
#[inline]
pub fn handle_communication_err(
    peer_addr: SocketAddr,
    e: &Error,
    details: &str,
    unsent_user_msg: Option<bytes::Bytes>,
) {
    debug!(
        "ERROR in communication with peer {}: {:?} - {}. Details: {}",
        peer_addr, e, e, details
    );
    ctx_mut(|c| {
        let _ = c.connections.remove(&peer_addr);
        if let Some(m) = unsent_user_msg {
            let _ = c
                .event_tx
                .send(Event::UnsentUserMessage { peer_addr, msg: m });
        }
    });
}

/// Try reading from the disk into the given structure.
pub fn read_from_disk<D>(file_path: &Path) -> R<D>
where
    D: DeserializeOwned,
{
    Ok(File::open(file_path)
        .map_err(Into::into)
        .map(BufReader::new)
        .and_then(|mut rdr| bincode::deserialize_from(&mut rdr))?)
}

/// Try writing the given structure to the disk.
pub fn write_to_disk<S>(file_path: &Path, s: &S) -> R<()>
where
    S: Serialize,
{
    File::create(file_path)
        .map_err(Into::into)
        .map(BufWriter::new)
        .and_then(|mut rdr| bincode::serialize_into(&mut rdr, s))?;

    Ok(())
}
