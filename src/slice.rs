//! Module handling lazy loading via iterating on slices on the original buffer.
use crate::lib::Vec;
use crate::x8d_tensor::TensorView;  // Changed from safetensors to x8dsub-byte
use core::fmt::Display;
use core::ops::{
    Bound, Range, RangeBounds, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

/// Error representing invalid slicing attempt
#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum InvalidSlice {
    /// When the client asked for more slices than the tensors has dimensions
    TooManySlices,
    /// When the client asked for a slice that exceeds the allowed bounds
    SliceOutOfRange {
        /// The rank of the dimension that has the out of bounds
        dim_index: usize,
        /// The problematic value
        asked: usize,
        /// The dimension size we shouldn't go over.
        dim_size: usize,
    },
    /// For smaller than 1 byte dtypes, some slices will happen outside of the byte boundary, some special care has to be taken
    /// and standard functions will fail
    MisalignedSlice,
}

impl Display for InvalidSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            InvalidSlice::TooManySlices => {
                write!(f, "more slicing indexes than dimensions in tensor")
            }
            InvalidSlice::SliceOutOfRange {
                dim_index,
                asked,
                dim_size,
            } => {
                write!(f, "index {asked} out of bounds for tensor dimension #{dim_index} of size {dim_size}")
            }
            InvalidSlice::MisalignedSlice => {
                write!(f, "The slice is slicing for subbytes dtypes, and the slice does not end up at a byte boundary, this is invalid.")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidSlice {}

/// Trait for indexing operations on tensors (e.g., `tensor[5]` or `tensor[2..7]`).
pub trait TensorIndexer {}  // Keep this name for compatibility

impl TensorIndexer for usize {}
impl TensorIndexer for Range<usize> {}
impl TensorIndexer for RangeInclusive<usize> {}
impl TensorIndexer for RangeFrom<usize> {}
impl TensorIndexer for RangeTo<usize> {}
impl TensorIndexer for RangeToInclusive<usize> {}
impl TensorIndexer for std::ops::RangeFull {}

/// The struct that combines multiple indexer to index a tensor
#[derive(Debug, Clone)]
pub struct TensorIndexer {
    pub(crate) indexer: Vec<IndexOp>,
}

impl TensorIndexer {
    pub(crate) fn new() -> Self {
        Self { indexer: vec![] }
    }

    #[allow(clippy::should_implement_trait)]
    pub(crate) fn mul(mut self, op: IndexOp) -> Self {
        self.indexer.push(op);
        self
    }
}

impl<I: TensorIndexer> core::ops::Mul<I> for TensorIndexer {
    type Output = TensorIndexer;

    fn mul(self, rhs: I) -> Self::Output {
        self.mul(IndexOp::from(rhs))
    }
}

impl<I: TensorIndexer> core::ops::Mul<I> for &TensorIndexer {
    type Output = TensorIndexer;

    fn mul(self, rhs: I) -> Self::Output {
        self.clone().mul(IndexOp::from(rhs))
    }
}

impl From<usize> for IndexOp {
    fn from(index: usize) -> Self {
        IndexOp::Single(index)
    }
}

impl From<RangeFull> for IndexOp {
    fn from(_: RangeFull) -> Self {
        IndexOp::Slice(std::ops::RangeFull)
    }
}

impl From<Range<usize>> for IndexOp {
    fn from(index: Range<usize>) -> Self {
        IndexOp::Slice(index)
    }
}

impl From<RangeInclusive<usize>> for IndexOp {
    fn from(index: RangeInclusive<usize>) -> Self {
        IndexOp::Slice(index)
    }
}

impl From<RangeFrom<usize>> for IndexOp {
    fn from(index: RangeFrom<usize>) -> Self {
        IndexOp::Slice(index)
    }
}

impl From<RangeTo<usize>> for IndexOp {
    fn from(index: RangeTo<usize>) -> Self {
        IndexOp::Slice(index)
    }
}

impl From<RangeToInclusive<usize>> for IndexOp {
    fn from(index: RangeToInclusive<usize>) -> Self {
        IndexOp::Slice(index)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum IndexOp {
    Single(usize),
    Slice(std::ops::RangeFull),
    Slice(std::ops::Range<usize>),
    Slice(std::ops::RangeInclusive<usize>),
    Slice(std::ops::RangeFrom<usize>),
    Slice(std::ops::RangeTo<usize>),
    Slice(std::ops::RangeToInclusive<usize>),
}

impl From<RangeFull> for IndexOp {
    fn from(_: RangeFull) -> Self {
        IndexOp::Slice(std::ops::RangeFull)
    }
}

impl From<Range<usize>> for IndexOp {
    fn from(range: Range<usize>) -> Self {
        IndexOp::Slice(range)
    }
}

impl From<RangeInclusive<usize>> for IndexOp {
    fn from(range: RangeInclusive<usize>) -> Self {
        IndexOp::Slice(range)
    }
}

impl From<RangeFrom<usize>> for IndexOp {
    fn from(range: RangeFrom<usize>) -> Self {
        IndexOp::Slice(range)
    }
}

impl From<RangeTo<usize>> for IndexOp {
    fn from(range: RangeTo<usize>) -> Self {
        IndexOp::Slice(range)
    }
}

impl From<RangeToInclusive<usize>> for IndexOp {
    fn from(range: RangeToInclusive<usize>) -> Self {
        IndexOp::Slice(range)
    }
}

impl From<usize> for IndexOp {
    fn from(index: usize) -> Self {
        IndexOp::Single(index)
    }
}

/// Iterator over a slice of a tensor
#[derive(Debug)]
pub struct SliceIterator<'data> {
    tensor: &'data TensorView<'data>,
    shape: Vec<usize>,
    strides: Vec<usize>,
    current: Vec<usize>,
    index: usize,
    n_elements: usize,
    element_size: usize,
}

impl<'data> SliceIterator<'data> {
    /// Creates a new SliceIterator
    /// This should be used internally by tensor only
    pub fn new(
        tensor: &'data TensorView<'data>,
        slices: &[TensorIndexer],
    ) -> Result<Self, InvalidSlice> {
        let shape = tensor.shape();
        if slices.len() > shape.len() {
            return Err(InvalidSlice::TooManySlices);
        }

        let mut new_shape: Vec<usize> = Vec::with_capacity(shape.len());
        let mut start_indices: Vec<usize> = Vec::with_capacity(shape.len());
        let mut end_indices: Vec<usize> = Vec::with_capacity(shape.len());

        // Process each dimension
        for (i, &dim_size) in shape.iter().enumerate() {
            let range = if i < slices.len() {
                // Apply the slice operation for this dimension
                let indexer = &slices[i];
                
                // Extract the range from the indexer
                let mut range = (0, dim_size); // Default to full range
                for op in &indexer.indexer {
                    match op {
                        IndexOp::Single(idx) => {
                            if *idx >= dim_size {
                                return Err(InvalidSlice::SliceOutOfRange {
                                    dim_index: i,
                                    asked: *idx,
                                    dim_size,
                                });
                            }
                            range = (*idx, *idx + 1) // Single index becomes range of size 1
                        },
                        IndexOp::Slice(range_bounds) => {
                            // Convert range bounds to actual range
                            let start = match range_bounds.start_bound() {
                                Bound::Included(&n) => n,
                                Bound::Excluded(&n) => n + 1,
                                Bound::Unbounded => 0,
                            };
                            let end = match range_bounds.end_bound() {
                                Bound::Included(&n) => n + 1,
                                Bound::Excluded(&n) => n,
                                Bound::Unbounded => dim_size,
                            };
                            
                            if start >= dim_size || end > dim_size || start > end {
                                let out_of_bounds_val = if start >= dim_size { start } else { end };
                                return Err(InvalidSlice::SliceOutOfRange {
                                    dim_index: i,
                                    asked: out_of_bounds_val,
                                    dim_size,
                                });
                            }
                            range = (start, end)
                        }
                    }
                }
                range
            } else {
                // Default to full range for unspecified dimensions
                (0, dim_size)
            };
            
            start_indices.push(range.0);
            end_indices.push(range.1);
            new_shape.push(range.1 - range.0);
        }

        // Calculate strides for the original tensor (not the sliced view)
        let mut strides: Vec<usize> = Vec::with_capacity(shape.len());
        let mut stride = 1;
        for &dim_size in shape.iter().rev() {
            strides.push(stride);
            stride *= dim_size;
        }
        strides.reverse();

        // Calculate total number of elements in the slice
        let n_elements = new_shape.iter().product();

        Ok(Self {
            tensor,
            shape: shape.to_vec(),  // Keep original shape for stride calculations
            strides,
            current: vec![0; new_shape.len()],  // Start with all zeros
            index: 0,
            n_elements,
            element_size: tensor.dtype().size(),
        })
    }
}

impl<'data> Iterator for SliceIterator<'data> {
    type Item = &'data [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.n_elements {
            return None;
        }

        // Calculate the linear index in the original data based on current position
        let mut linear_index = 0;
        for (i, &current_pos) in self.current.iter().enumerate() {
            // Adjust current position by the slice start for this dimension
            let actual_pos = self.shape[i] - (self.shape[i] - self.current[i]); // This is just current_pos
            linear_index += actual_pos * self.strides[i];
        }

        // Calculate the size of each element based on dtype
        let element_size = self.tensor.dtype().size();

        // Get the slice of data for this element
        let start = linear_index * element_size;
        let end = start + element_size;

        // Update current position for next iteration
        let mut carry = 1;
        for i in (0..self.current.len()).rev() {
            self.current[i] += carry;
            // Check if we've exceeded the range for this dimension
            if self.current[i] >= (self.shape[i] - (self.shape[i] - (self.shape[i]))) {  // Simplified to self.current[i] >= self.shape[i]
                self.current[i] = 0;
                carry = 1;
            } else {
                carry = 0;
                break;
            }
        }

        self.index += 1;

        if start < self.tensor.data().len() && end <= self.tensor.data().len() {
            Some(&self.tensor.data()[start..end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x8d_tensor::{Dtype, TensorView};  // Updated import

    #[test]
    fn test_single_element_slice() {
        let data = vec![0u8; 24]; // 2*3*4 elements of u8
        let tensor = TensorView::new(Dtype::U8, vec![2, 3, 4], &data).unwrap();
        let slices = [TensorIndexer::new().mul(0usize.into())]; // Get first dimension
        let mut iter = SliceIterator::new(&tensor, &slices).unwrap();
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_range_slice() {
        let data = vec![0u8; 24]; // 2*3*4 elements of u8
        let tensor = TensorView::new(Dtype::U8, vec![2, 3, 4], &data).unwrap();
        let range = 1..3;
        let slices = [TensorIndexer::new().mul(range.into())];
        let mut iter = SliceIterator::new(&tensor, &slices).unwrap();
        assert!(iter.next().is_some());
        assert!(iter.next().is_some());
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_out_of_bounds() {
        let data = vec![0u8; 24]; // 2*3*4 elements of u8
        let tensor = TensorView::new(Dtype::U8, vec![2, 3, 4], &data).unwrap();
        let slices = [TensorIndexer::new().mul(5usize.into())]; // Index 5 is out of bounds for dim of size 2
        let result = SliceIterator::new(&tensor, &slices);
        assert!(result.is_err());
        match result.unwrap_err() {
            InvalidSlice::SliceOutOfRange { dim_index, asked, dim_size } => {
                assert_eq!(dim_index, 0);
                assert_eq!(asked, 5);
                assert_eq!(dim_size, 2);
            }
            _ => panic!("Wrong error type"),
        }
    }
}