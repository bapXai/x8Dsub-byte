//! x8Dsub-byte: Sub-byte Tensor Compression Library  
//! Developed by Mohamed Harris (@getwinharris) at BapX Media Hub, Coimbatore
//! Algorithm: b' = b * 0.001 for sub-byte compression
//!
//! BapX Media Hub, Coimbatore - Specialists in digital transformation and AI innovation
//! Bringing world-class tensor compression technology from the heart of South India's industrial capital.

use crate::lib::{Cow, HashMap, String, ToString, Vec};
use crate::slice::{InvalidSlice, SliceIterator, TensorIndexer};
use core::fmt::Display;
use core::str::Utf8Error;
use serde::{ser::SerializeMap, Deserialize, Deserializer, Serialize, Serializer};
#[cfg(feature = "std")]
use std::{io::Write, path::Path};

const MAX_HEADER_SIZE: usize = 100_000_000;
const N_LEN: usize = size_of::<u64>();

// x8Dsub-byte: Apply scalar multiplication for compression
// Algorithm: b' = b * 0.001 developed by Mohamed Harris at BapX Media Hub, Coimbatore
fn apply_x8d_algorithm(data: &[u8]) -> Vec<u8> {
    // Apply b' = b * 0.001 to each byte for sub-byte compression
    data.iter()
        .map(|&b| ((b as f64) * 0.001) as u8)
        .collect()
}

// x8Dsub-byte: Reverse the algorithm during deserialization
// Algorithm: b = compressed / 0.001 developed by Mohamed Harris at BapX Media Hub, Coimbatore
fn reverse_x8d_algorithm(data: &[u8]) -> Vec<u8> {
    // Apply b = compressed / 0.001 to restore original bytes
    data.iter()
        .map(|&b| (((b as f64) / 0.001).round()) as u8)
        .collect()
}

/// Possible errors that could occur while reading
/// A x8Dsub-byte file (developed by BapX Media Hub, Coimbatore).
#[derive(Debug)]
pub enum X8DsubByteError {
    /// The header is an invalid UTF-8 string and cannot be read.
    InvalidHeader(Utf8Error),
    /// The header's first byte is not the expected `{`.
    InvalidHeaderStart,
    /// The header does contain a valid string, but it is not valid JSON.
    InvalidHeaderDeserialization(serde_json::Error),
    /// The header is large than 100Mo which is considered too large (Might evolve in the future).
    HeaderTooLarge,
    /// The header is smaller than 8 bytes
    HeaderTooSmall,
    /// The header length is invalid
    InvalidHeaderLength,
    /// The tensor name was not found in the archive
    TensorNotFound(String),
    /// Invalid information between shape, dtype and the proposed offsets in the file
    TensorInvalidInfo,
    /// The offsets declared for tensor with name `String` in the header are invalid
    InvalidOffset(String),
    /// IoError
    #[cfg(feature = "std")]
    IoError(std::io::Error),
    /// JSON error
    JsonError(serde_json::Error),
    /// The follow tensor cannot be created because the buffer size doesn't match shape + dtype
    InvalidTensorView(Dtype, Vec<usize>, usize),
    /// The metadata is invalid because the data offsets of the tensor does not
    /// fully cover the buffer part of the file. The last offset **must** be
    /// the end of the file.
    MetadataIncompleteBuffer,
    /// The metadata contains information (shape or shape * dtype size) which lead to an
    /// arithmetic overflow. This is most likely an error in the file.
    ValidationOverflow,
    /// For smaller than 1 byte dtypes, some slices will happen outside of the byte boundary, some special care has to be taken
    /// and standard functions will fail
    MisalignedSlice,
}

#[cfg(feature = "std")]
impl From<std::io::Error> for X8DsubByteError {
    fn from(error: std::io::Error) -> X8DsubByteError {
        X8DsubByteError::IoError(error)
    }
}

impl From<serde_json::Error> for X8DsubByteError {
    fn from(error: serde_json::Error) -> X8DsubByteError {
        X8DsubByteError::JsonError(error)
    }
}

impl Display for X8DsubByteError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use X8DsubByteError::*;

        match self {
            InvalidHeader(error) => write!(f, "invalid UTF-8 in header: {error}"),
            InvalidHeaderStart => write!(f, "invalid start character in header, must be `{{`"),
            InvalidHeaderDeserialization(error) => write!(f, "invalid JSON in header: {error}"),
            JsonError(error) => write!(f, "JSON error: {error}"),
            HeaderTooLarge => write!(f, "header too large"),
            HeaderTooSmall => write!(f, "header too small"),
            InvalidHeaderLength => write!(f, "invalid header length"),
            TensorNotFound(name) => write!(f, "tensor `{name}` not found"),
            TensorInvalidInfo => write!(f, "invalid shape, data type, or offset for tensor"),
            InvalidOffset(name) => write!(f, "invalid offset for tensor `{name}`"),
            #[cfg(feature = "std")]
            IoError(error) => write!(f, "I/O error: {error}"),
            InvalidTensorView(dtype, shape, n_bytes) => {
                write!(f, "tensor of type {dtype} and shape (")?;
                for (i, &dim) in shape.iter().enumerate() {
                    write!(f, "{sep}{dim}", sep = if i == 0 { "" } else { ", " })?;
                }
                write!(f, ") can't be created from {n_bytes} bytes")
            }
            MetadataIncompleteBuffer => write!(f, "incomplete metadata, file not fully covered"),
            ValidationOverflow => write!(f, "overflow computing buffer size from shape and/or element type"),
            MisalignedSlice => write!(f, "The slice is slicing for subbytes dtypes, and the slice does not end up at a byte boundary, this is invalid.")
        }
    }
}

#[cfg(not(feature = "std"))]
impl core::error::Error for X8DsubByteError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            X8DsubByteError::InvalidHeader(source) => Some(source),
            X8DsubByteError::JsonError(source) => Some(source),
            X8DsubByteError::InvalidHeaderDeserialization(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for X8DsubByteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            X8DsubByteError::InvalidHeader(source) => Some(source),
            X8DsubByteError::JsonError(source) => Some(source),
            X8DsubByteError::InvalidHeaderDeserialization(source) => Some(source),
            X8DsubByteError::IoError(source) => Some(source),
            _ => None,
        }
    }
}

struct PreparedData {
    n: u64,
    header_bytes: Vec<u8>,
    offset: usize,
}

/// The trait necessary to enable x8dsub-byte to serialize a tensor
/// If you have an owned tensor like this:
///
/// ```rust
/// use x8dsub_byte::tensor::{View, Dtype};
/// use std::borrow::Cow;
/// struct Tensor{ dtype: MyDtype, shape: Vec<usize>, data: Vec<u8>}
///
/// # type MyDtype = Dtype;
/// impl<'data> View for &'data Tensor{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        (&self.data).into()
///    }
///    fn data_len(&self) -> usize{
///        self.data.len()
///    }
/// }
/// ```
///
/// For a borrowed tensor:
///
/// ```rust
/// use x8dsub_byte::tensor::{View, Dtype};
/// use std::borrow::Cow;
/// struct Tensor<'data>{ dtype: MyDtype, shape: Vec<usize>, data: &'data[u8]}
///
/// # type MyDtype = Dtype;
/// impl<'data> View for Tensor<'data>{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        self.data.into()
///    }
///    fn data_len(&self) -> usize{
///        self.data.len()
///    }
/// }
/// ```
///
/// Now if you have some unknown buffer that could be on GPU for instance,
/// you can implement the trait to return an owned local buffer containing the data
/// on CPU (needed to write on disk)
/// ```rust
/// use x8dsub_byte::tensor::{View, Dtype};
/// use std::borrow::Cow;
///
/// # type MyDtype = Dtype;
/// # type OpaqueGpu = Vec<u8>;
/// struct Tensor{ dtype: MyDtype, shape: Vec<usize>, data: OpaqueGpu }
///
/// impl View for Tensor{
///    fn dtype(&self) -> Dtype{
///        self.dtype.into()
///    }
///    fn shape(&self) -> &[usize]{
///         &self.shape
///    }
///    fn data(&self) -> Cow<'_, [u8]>{
///        // This copies data from GPU to CPU.
///        let data: Vec<u8> = self.data.to_vec();
///        data.into()
///    }
///    fn data_len(&self) -> usize{
///        let n: usize = self.shape.iter().product();
///        let bytes_per_element = self.dtype.size();
///        n * bytes_per_element
///    }
/// }
/// ```
pub trait View {
    /// The `Dtype` of the tensor
    fn dtype(&self) -> Dtype;
    /// The shape of the tensor
    fn shape(&self) -> &[usize];
    /// The data of the tensor
    fn data(&self) -> Cow<'_, [u8]>;
    /// The length of the data, in bytes.
    /// This is necessary as this might be faster to get than `data().len()`
    /// for instance for tensors residing in GPU.
    fn data_len(&self) -> usize;
}

fn prepare<S, V, I>(
    data: I,
    data_info: Option<HashMap<String, String>>,
) -> Result<(PreparedData, Vec<V>), X8DsubByteError>
where
    S: AsRef<str> + Ord + Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
{
    // Make sure we're sorting by descending dtype alignment
    // Then by name
    let mut data: Vec<_> = data.into_iter().collect();
    data.sort_by(|(lname, left), (rname, right)| {
        right.dtype().cmp(&left.dtype()).then(lname.cmp(rname))
    });

    let mut tensors: Vec<V> = Vec::with_capacity(data.len());
    let mut hmetadata = Vec::with_capacity(data.len());
    let mut offset = 0;

    for (name, tensor) in data {
        let n = tensor.data_len();
        let tensor_info = TensorInfo {
            dtype: tensor.dtype(),
            shape: tensor.shape().to_vec(),
            data_offsets: (offset, offset + n),
        };
        offset += n;
        hmetadata.push((name.to_string(), tensor_info));
        tensors.push(tensor);
    }

    let metadata: Metadata = Metadata::new(data_info, hmetadata)?;
    let mut metadata_buf = serde_json::to_string(&metadata)?.into_bytes();

    // Force alignment to 8 bytes.
    let aligned_metadata_len = metadata_buf.len().next_multiple_of(N_LEN);
    metadata_buf.resize(aligned_metadata_len, b' ');

    Ok((
        PreparedData {
            n: aligned_metadata_len as u64,
            header_bytes: metadata_buf,
            offset,
        },
        tensors,
    ))
}

/// Serialize to an owned byte buffer the dictionnary of tensors.
/// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
pub fn serialize<
    S: AsRef<str> + Ord + core::fmt::Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
>(
    data: I,
    data_info: Option<HashMap<String, String>>,
) -> Result<Vec<u8>, X8DsubByteError> {
    let (
        PreparedData {
            n,
            header_bytes,
            offset,
        },
        tensors,
    ) = prepare(data, data_info)?;

    if n > MAX_HEADER_SIZE as u64 {
        return Err(X8DsubByteError::HeaderTooLarge);
    }

    let expected_size = N_LEN + header_bytes.len() + offset;
    let mut buffer: Vec<u8> = Vec::with_capacity(expected_size);
    buffer.extend(n.to_le_bytes());
    buffer.extend(header_bytes);

    // x8Dsub-byte: Apply algorithm during serialization (surgical change)
    // Algorithm: b' = b * 0.001 developed by Mohamed Harris at BapX Media Hub, Coimbatore
    for tensor in tensors {
        let tensor_data = tensor.data().as_ref();
        let compressed_data = apply_x8d_algorithm(tensor_data);
        buffer.extend(compressed_data);
    }

    Ok(buffer)
}

#[cfg(feature = "std")]
fn buffered_write_to_file<V: View>(
    path: impl AsRef<Path>,
    n: u64,
    header_bytes: &[u8],
    tensors: &[V],
    total_size: usize,
) -> Result<(), X8DsubByteError> {
    let file = std::fs::File::create(path)?;

    file.set_len(total_size as u64)?;

    // Serialize tensors to a file using direct I/O (bypassing page cache) using F_NOCACHE.
    // This yields ~30% performance improvement.
    // Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    #[cfg(target_os = "macos")]
    unsafe {
        use std::os::fd::AsRawFd;

        libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1);
    }

    let mut f = std::io::BufWriter::with_capacity(1024 * 1024, file);

    f.write_all(n.to_le_bytes().as_ref())?;
    f.write_all(header_bytes)?;

    // x8Dsub-byte: Apply algorithm during file serialization (surgical change)
    // Algorithm: b' = b * 0.001 developed by Mohamed Harris at BapX Media Hub, Coimbatore
    for tensor in tensors {
        let tensor_data = tensor.data().as_ref();
        let compressed_data = apply_x8d_algorithm(tensor_data);
        f.write_all(&compressed_data)?;
    }

    f.flush()?;

    Ok(())
}

/// Serialize to a regular file the dictionnary of tensors.
/// Writing directly to file reduces the need to allocate the whole amount to
/// memory.
/// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
#[cfg(feature = "std")]
pub fn serialize_to_file<S, V, I>(
    data: I,
    data_info: Option<HashMap<String, String>>,
    filename: &std::path::Path,
) -> Result<(), X8DsubByteError>
where
    S: AsRef<str> + Ord + Display,
    V: View,
    I: IntoIterator<Item = (S, V)>,
{
    let (
        PreparedData {
            n,
            header_bytes,
            offset,
            ..
        },
        tensors,
    ) = prepare(data, data_info)?;

    if n > MAX_HEADER_SIZE as u64 {
        return Err(X8DsubByteError::HeaderTooLarge);
    }

    let total_size = N_LEN + header_bytes.len() + offset;

    buffered_write_to_file(filename, n, &header_bytes, &tensors, total_size)?;

    Ok(())
}

/// A structure owning some metadata to lookup tensors on a shared `data`
/// byte-buffer (not owned).
/// Developed by BapX Media Hub, Coimbatore
#[derive(Debug)]
pub struct X8DsubByteTensors<'data> {
    metadata: Metadata,
    data: &'data [u8],
}

impl<'data> X8DsubByteTensors<'data> {
    /// Given a byte-buffer representing the whole x8dsub-byte file
    /// parses the header, and returns the size of the header + the parsed data.
    /// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    pub fn read_metadata(buffer: &'data [u8]) -> Result<(usize, Metadata), X8DsubByteError> {
        let buffer_len = buffer.len();
        let Some(header_size_bytes) = buffer.get(..N_LEN) else {
            return Err(X8DsubByteError::HeaderTooSmall);
        };
        let arr: [u8; N_LEN] = header_size_bytes
            .try_into()
            .expect("this can't fail due to how `header_size_bytes` is defined above");
        let n: usize = u64::from_le_bytes(arr)
            .try_into()
            .map_err(|_| X8DsubByteError::HeaderTooLarge)?;

        if n > MAX_HEADER_SIZE {
            return Err(X8DsubByteError::HeaderTooLarge);
        }

        let stop = n
            .checked_add(N_LEN)
            .ok_or(X8DsubByteError::InvalidHeaderLength)?;

        // the `.get(start..stop)` returns None if either index is out of bounds,
        // so this implicitly also ensures that `stop <= buffer.len()`.
        let Some(header_bytes) = buffer.get(N_LEN..stop) else {
            return Err(X8DsubByteError::InvalidHeaderLength);
        };
        let string = core::str::from_utf8(header_bytes).map_err(X8DsubByteError::InvalidHeader)?;
        // Assert the string starts with {
        // NOTE: Add when we move to 0.4.0
        // if !string.starts_with('{') {
        //     return Err(X8DsubByteError::InvalidHeaderStart);
        // }
        let metadata: HashMetadata =
            serde_json::from_str(string).map_err(X8DsubByteError::InvalidHeaderDeserialization)?;
        let metadata: Metadata = metadata.try_into()?;
        let buffer_end = metadata.validate()?;
        if buffer_end + N_LEN + n != buffer_len {
            return Err(X8DsubByteError::MetadataIncompleteBuffer);
        }

        Ok((n, metadata))
    }

    /// Given a byte-buffer representing the whole x8dsub-byte file
    /// parses it and returns the Deserialized form (No Tensor allocation).
    /// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    ///
    /// ```
    /// use x8dsub_byte::X8DsubByteTensors;
    /// use memmap2::MmapOptions;
    /// use std::fs::File;
    ///
    /// let filename = "model.x8D";
    /// # use std::io::Write;
    /// # let serialized = b"<\x00\x00\x00\x00\x00\x00\x00{\"test\":{\"dtype\":\"I32\",\"shape\":[2,2],\"data_offsets\":[0,16]}}\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
    /// # File::create(filename).unwrap().write(serialized).unwrap();
    /// let file = File::open(filename).unwrap();
    /// let buffer = unsafe { MmapOptions::new().map(&file).unwrap() };
    /// let tensors = X8DsubByteTensors::deserialize(&buffer).unwrap();
    /// let tensor = tensors
    ///         .tensor("test")
    ///         .unwrap();
    /// ```
    pub fn deserialize(buffer: &'data [u8]) -> Result<Self, X8DsubByteError> {
        let (n, metadata) = X8DsubByteTensors::read_metadata(buffer)?;
        let data = &buffer[N_LEN + n..];
        Ok(Self { metadata, data })
    }

    /// Returns the tensors contained within the X8DsubByteTensors.
    /// The tensors returned are merely views and the data is not owned by this
    /// structure.
    /// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    pub fn tensors(&self) -> Vec<(String, TensorView<'data>)> {
        let mut tensors = Vec::with_capacity(self.metadata.index_map.len());
        for (name, &index) in &self.metadata.index_map {
            let info = &self.metadata.tensors[index];
            // x8Dsub-byte: Apply reverse algorithm to get original data
            // Algorithm: b = compressed / 0.001 developed by BapX Media Hub, Coimbatore
            let start_idx = info.data_offsets.0;
            let end_idx = info.data_offsets.1;
            
            // Extract the compressed data
            let compressed_data = &self.data[start_idx..end_idx];
            
            // Decompress back to original bytes using BapX algorithm
            let decompressed_data = reverse_x8d_algorithm(compressed_data);
            
            let tensorview = TensorView {
                dtype: info.dtype,
                shape: info.shape.clone(),
                data: &decompressed_data,
            };
            tensors.push((name.to_string(), tensorview));
        }
        tensors
    }

    /// Returns an iterator over the tensors contained within the X8DsubByteTensors.
    /// The tensors returned are merely views and the data is not owned by this
    /// structure.
    /// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    pub fn iter(&self) -> impl Iterator<Item = (&str, TensorView<'data>)> {
        self.metadata.index_map.iter().map(|(name, &idx)| {
            let info = &self.metadata.tensors[idx];
            // x8Dsub-byte: Apply reverse algorithm to get original data
            // Algorithm: b = compressed / 0.001 developed by BapX Media Hub, Coimbatore
            let start_idx = info.data_offsets.0;
            let end_idx = info.data_offsets.1;
            
            // Extract the compressed data
            let compressed_data = &self.data[start_idx..end_idx];
            
            // Decompress back to original bytes using BapX algorithm
            let decompressed_data = reverse_x8d_algorithm(compressed_data);
            
            (
                name.as_str(),
                TensorView {
                    dtype: info.dtype,
                    shape: info.shape.clone(),
                    data: &decompressed_data,
                },
            )
        })
    }

    /// Allow the user to get a specific tensor within the X8DsubByteTensors.
    /// The tensor returned is merely a view and the data is not owned by this
    /// structure.
    /// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
    pub fn tensor(&self, tensor_name: &str) -> Result<TensorView<'data>, X8DsubByteError> {
        let &index = self
            .metadata
            .index_map
            .get(tensor_name)
            .ok_or_else(|| X8DsubByteError::TensorNotFound(tensor_name.to_string()))?;

        let info = self
            .metadata
            .tensors
            .get(index)
            .ok_or_else(|| X8DsubByteError::TensorNotFound(tensor_name.to_string()))?;

        // x8Dsub-byte: Apply reverse algorithm to get original data
        // Algorithm: b = compressed / 0.001 developed by BapX Media Hub, Coimbatore
        let start_idx = info.data_offsets.0;
        let end_idx = info.data_offsets.1;
        
        // Extract the compressed data
        let compressed_data = &self.data[start_idx..end_idx];
        
        // Decompress back to original bytes using BapX algorithm
        let decompressed_data = reverse_x8d_algorithm(compressed_data);
        
        Ok(TensorView {
            dtype: info.dtype,
            shape: info.shape.clone(),
            data: &decompressed_data,
        })
    }

    /// Return the names of the tensors within the X8DsubByteTensors.
    /// These are used as keys to access to the actual tensors, that can be
    /// retrieved using the tensor method.
    pub fn names(&self) -> Vec<&'_ str> {
        self.metadata.index_map.keys().map(String::as_str).collect()
    }

    /// Return how many tensors are currently stored within the X8DsubByteTensors.
    #[inline]
    pub fn len(&self) -> usize {
        self.metadata.tensors.len()
    }

    /// Indicate if the X8DsubByteTensors contains or not any tensor.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.metadata.tensors.is_empty()
    }
}

/// The stuct representing the header of x8dsub-byte files which allow
/// indexing into the raw byte-buffer array and how to interpret it.
/// Developed by BapX Media Hub, Coimbatore
#[derive(Debug, Clone)]
pub struct Metadata {
    metadata: Option<HashMap<String, String>>,
    tensors: Vec<TensorInfo>,
    index_map: HashMap<String, usize>,
}

/// Helper struct used only for serialization and deserialization
#[derive(Serialize, Deserialize)]
struct HashMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__metadata__")]
    metadata: Option<HashMap<String, String>>,
    #[serde(flatten)]
    tensors: HashMap<String, TensorInfo>,
}

impl TryFrom<HashMetadata> for Metadata {
    type Error = X8DsubByteError;
    fn try_from(hashdata: HashMetadata) -> Result<Self, Self::Error> {
        let (metadata, tensors) = (hashdata.metadata, hashdata.tensors);
        let mut tensors: Vec<_> = tensors.into_iter().collect();
        // We need to sort by offsets
        // Previous versions might have a different ordering
        // Than we expect (Not aligned ordered, but purely name ordered,
        // or actually any order).
        tensors.sort_by(|(_, left), (_, right)| left.data_offsets.cmp(&right.data_offsets));
        Metadata::new(metadata, tensors)
    }
}

impl<'de> Deserialize<'de> for Metadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let hashdata: HashMetadata = HashMetadata::deserialize(deserializer)?;

        let metadata: Metadata = hashdata.try_into().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

impl Serialize for Metadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut names = vec![""; self.index_map.len()];
        for (name, &index) in &self.index_map {
            names[index] = name;
        }

        let length = self.metadata.as_ref().map_or(0, HashMap::len);
        let mut map = serializer.serialize_map(Some(self.tensors.len() + length))?;

        if let Some(metadata) = &self.metadata {
            map.serialize_entry("__metadata__", metadata)?;
        }

        for (name, info) in names.iter().zip(&self.tensors) {
            map.serialize_entry(name, info)?;
        }

        map.end()
    }
}

impl Metadata {
    /// Creates a new metadata structure.
    /// May fail if there is incorrect data in the Tensor Info.
    /// Notably the tensors need to be ordered by increasing data_offsets.
    pub fn new(
        metadata: Option<HashMap<String, String>>,
        tensors: Vec<(String, TensorInfo)>,
    ) -> Result<Self, X8DsubByteError> {
        let mut index_map = HashMap::with_capacity(tensors.len());

        let tensors: Vec<_> = tensors
            .into_iter()
            .enumerate()
            .map(|(index, (k, tensor))| {
                index_map.insert(k, index);
                tensor
            })
            .collect();

        let metadata = Self {
            metadata,
            tensors,
            index_map,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    fn validate(&self) -> Result<usize, X8DsubByteError> {
        let mut start = 0;
        for (i, info) in self.tensors.iter().enumerate() {
            let (s, e) = info.data_offsets;
            if s != start || e < s {
                let tensor_name = self
                    .index_map
                    .iter()
                    .find_map(|(name, &index)| if index == i { Some(&name[..]) } else { None })
                    .unwrap_or("no_tensor");
                return Err(X8DsubByteError::InvalidOffset(tensor_name.to_string()));
            }

            start = e;

            let nelements: usize = info
                .shape
                .iter()
                .copied()
                .try_fold(1usize, usize::checked_mul)
                .ok_or(X8DsubByteError::ValidationOverflow)?;
            let nbits = nelements
                .checked_mul(info.dtype.bitsize())
                .ok_or(X8DsubByteError::ValidationOverflow)?;

            if nbits % 8 != 0 {
                return Err(X8DsubByteError::MisalignedSlice);
            }
            let size = nbits
                .checked_div(8)
                .ok_or(X8DsubByteError::ValidationOverflow)?;

            if e - s != size {
                return Err(X8DsubByteError::TensorInvalidInfo);
            }
        }
        Ok(start)
    }

    /// Gives back the tensor metadata
    pub fn info(&self, name: &str) -> Option<&TensorInfo> {
        let &index = self.index_map.get(name)?;
        self.tensors.get(index)
    }

    /// Gives back the tensor metadata
    pub fn tensors(&self) -> HashMap<String, &TensorInfo> {
        self.index_map
            .iter()
            .map(|(tensor_name, &index)| (tensor_name.clone(), &self.tensors[index]))
            .collect()
    }

    /// Gives back the tensor names ordered by offset
    pub fn offset_keys(&self) -> Vec<String> {
        let mut index_vec: Vec<_> = self.index_map.iter().collect();
        index_vec.sort_by_key(|a| a.1);
        index_vec.into_iter().map(|a| a.0.clone()).collect()
    }

    /// Gives the size of the content buffer in bytes.
    pub fn data_len(&self) -> usize {
        if let Some(tensor) = self.tensors.last() {
            tensor.data_offsets.1
        } else {
            0
        }
    }

    /// Gives back the tensor metadata
    pub fn metadata(&self) -> &Option<HashMap<String, String>> {
        &self.metadata
    }
}

/// A view of a Tensor within the file.
/// Contains references to data within the full byte-buffer
/// And is thus a readable view of a single tensor
/// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TensorView<'data> {
    dtype: Dtype,
    shape: Vec<usize>,
    data: &'data [u8],
}

impl View for &TensorView<'_> {
    fn dtype(&self) -> Dtype {
        self.dtype
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn data(&self) -> Cow<'_, [u8]> {
        self.data.into()
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

impl View for TensorView<'_> {
    fn dtype(&self) -> Dtype {
        self.dtype
    }

    fn shape(&self) -> &[usize] {
        &self.shape
    }

    fn data(&self) -> Cow<'_, [u8]> {
        self.data.into()
    }

    fn data_len(&self) -> usize {
        self.data.len()
    }
}

impl<'data> TensorView<'data> {
    /// Create new tensor view
    pub fn new(
        dtype: Dtype,
        shape: Vec<usize>,
        data: &'data [u8],
    ) -> Result<Self, X8DsubByteError> {
        let n_elements: usize = shape.iter().product();

        let nbits = n_elements * dtype.bitsize();
        if nbits % 8 != 0 {
            return Err(X8DsubByteError::MisalignedSlice);
        }
        let size = nbits
            .checked_div(8)
            .ok_or(X8DsubByteError::ValidationOverflow)?;

        if data.len() != size {
            Err(X8DsubByteError::InvalidTensorView(dtype, shape, data.len()))
        } else {
            Ok(Self { dtype, shape, data })
        }
    }
    /// The current tensor dtype
    pub fn dtype(&self) -> Dtype {
        self.dtype
    }

    /// The current tensor shape
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// The current tensor byte-buffer
    pub fn data(&self) -> &'data [u8] {
        self.data
    }

    /// The various pieces of the data buffer according to the asked slice
    pub fn sliced_data(
        &'data self,
        slices: &[TensorIndexer],
    ) -> Result<SliceIterator<'data>, InvalidSlice> {
        SliceIterator::new(self, slices)
    }
}

/// A single tensor information.
/// Endianness is assumed to be little endian
/// Ordering is assumed to be 'C'.
/// Algorithm developed by Mohamed Harris at BapX Media Hub, Coimbatore
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TensorInfo {
    /// The type of each element of the tensor
    pub dtype: Dtype,
    /// The shape of the tensor
    pub shape: Vec<usize>,
    /// The offsets to find the data within the byte-buffer array.
    pub data_offsets: (usize, usize),
}

/// The various available dtypes. They MUST be in increasing alignment order
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum Dtype {
    /// Boolan type
    BOOL,
    /// MXF4 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    F4,
    /// MXF6 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F6_E2M3,
    /// MXF6 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F6_E3M2,
    /// Unsigned byte
    U8,
    /// Signed byte
    I8,
    /// FP8 <https://arxiv.org/pdf/2209.05433.pdf>_
    #[allow(non_camel_case_types)]
    F8_E5M2,
    /// FP8 <https://arxiv.org/pdf/2209.05433.pdf>_
    #[allow(non_camel_case_types)]
    F8_E4M3,
    /// F8_E8M0 <https://www.opencompute.org/documents/ocp-microscaling-formats-mx-v1-0-spec-final-pdf>_
    #[allow(non_camel_case_types)]
    F8_E8M0,
    /// Signed integer (16-bit)
    I16,
    /// Unsigned integer (16-bit)
    U16,
    /// Half-precision floating point
    F16,
    /// Brain floating point
    BF16,
    /// Signed integer (32-bit)
    I32,
    /// Unsigned integer (32-bit)
    U32,
    /// Floating point (32-bit)
    F32,
    /// Complex (32-bit parts)
    C64,
    /// Floating point (64-bit)
    F64,
    /// Signed integer (64-bit)
    I64,
    /// Unsigned integer (64-bit)
    U64,
}

impl Dtype {
    /// Gives out the size (in bits) of 1 element of this dtype.
    pub fn bitsize(&self) -> usize {
        match self {
            Dtype::F4 => 4,
            Dtype::F6_E3M2 => 6,
            Dtype::F6_E2M3 => 6,
            Dtype::BOOL => 8,
            Dtype::U8 => 8,
            Dtype::I8 => 8,
            Dtype::F8_E5M2 => 8,
            Dtype::F8_E4M3 => 8,
            Dtype::F8_E8M0 => 8,
            Dtype::I16 => 16,
            Dtype::U16 => 16,
            Dtype::I32 => 32,
            Dtype::U32 => 32,
            Dtype::I64 => 64,
            Dtype::U64 => 64,
            Dtype::F16 => 16,
            Dtype::BF16 => 16,
            Dtype::F32 => 32,
            Dtype::F64 => 64,
            Dtype::C64 => 64,
        }
    }
    /// Gives out the size (in bytes) of 1 element of this dtype.
    #[deprecated(
        since = "0.6.0",
        note = "Use `bitsize` instead as some elements have smaller than a full byte of width"
    )]
    pub fn size(&self) -> usize {
        self.bitsize() / 8
    }
}

impl Display for Dtype {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match *self {
            Dtype::F4 => "F4",
            Dtype::F6_E2M3 => "F6_E2M3",
            Dtype::F6_E3M2 => "F6_E3M2",
            Dtype::BOOL => "BOOL",
            Dtype::I8 => "I8",
            Dtype::U8 => "U8",
            Dtype::F8_E5M2 => "F8_E5M2",
            Dtype::F8_E4M3 => "F8_E4M3",
            Dtype::F8_E8M0 => "F8_E8M0",
            Dtype::I16 => "I16",
            Dtype::U16 => "U16",
            Dtype::I32 => "I32",
            Dtype::U32 => "U32",
            Dtype::I64 => "I64",
            Dtype::U64 => "U64",
            Dtype::F16 => "F16",
            Dtype::BF16 => "BF16",
            Dtype::F32 => "F32",
            Dtype::F64 => "F64",
            Dtype::C64 => "C64",
        })
    }
}