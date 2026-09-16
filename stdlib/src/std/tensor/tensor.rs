//! Generic Tensor structure with QTT quantity enforcement

use super::*;
use crate::core::prelude::*;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// QTT Quantity phantom type for enforcing linearity rules
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QttQuantity {
    Zero,   // [0] - Proof-only, zero-size shapes
    Linear, // [1] - Linear mutable buffers
    Heap,   // [*] - Heap storage
}

/// Marker types for QTT quantities
pub struct Q0; // [0] Proof-only
pub struct Q1; // [1] Linear
pub struct QStar; // [*] Heap

/// Type-level QTT quantity
pub trait QttQty: Sized + 'static {
    const QTY: QttQuantity;
    type Inner;
}

impl QttQty for Q0 {
    const QTY: QttQuantity = QttQuantity::Zero;
    type Inner = ();
}

impl QttQty for Q1 {
    const QTY: QttQuantity = QttQuantity::Linear;
    type Inner = ();
}

impl QttQty for QStar {
    const QTY: QttQuantity = QttQuantity::Heap;
    type Inner = ();
}

/// Generic tensor with QTT quantity parameter
/// 
/// Tensor<[Q]; Dims> where:
/// - Q0 = [0]: Proof-only, zero-size shapes (compile-time only, no runtime data)
/// - Q1 = [1]: Linear mutable buffers (unique ownership, move semantics)
/// - QStar = [*]: Heap storage (shared ownership, reference counted)
pub struct Tensor<Q: QttQty, D: Dims, T, L: Layout = RowMajor> {
    data: TensorStorage<Q, T>,
    shape: ConcreteShape,
    layout: L,
    _dims: PhantomData<D>,
}

/// Storage backend based on QTT quantity
enum TensorStorage<Q: QttQty, T> {
    Zero(PhantomData<T>),              // Q0: no storage
    Linear(Box<[T]>),                   // Q1: unique owned buffer
    Heap(std::sync::Arc<[T]>),          // QStar: shared reference counted
}

impl<Q: QttQty, T> TensorStorage<Q, T> {
    fn as_ptr(&self) -> *const T {
        match self {
            TensorStorage::Zero(_) => std::ptr::null(),
            TensorStorage::Linear(buf) => buf.as_ptr(),
            TensorStorage::Heap(buf) => buf.as_ptr(),
        }
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        match self {
            TensorStorage::Zero(_) => std::ptr::null_mut(),
            TensorStorage::Linear(buf) => buf.as_mut_ptr(),
            TensorStorage::Heap(buf) => std::sync::Arc::get_mut(buf).map(|b| b.as_mut_ptr()).unwrap_or(std::ptr::null_mut()),
        }
    }

    fn len(&self) -> usize {
        match self {
            TensorStorage::Zero(_) => 0,
            TensorStorage::Linear(buf) => buf.len(),
            TensorStorage::Heap(buf) => buf.len(),
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<Q: QttQty, T: Clone> Clone for TensorStorage<Q, T> {
    fn clone(&self) -> Self {
        match self {
            TensorStorage::Zero(_) => TensorStorage::Zero(PhantomData),
            TensorStorage::Linear(buf) => TensorStorage::Linear(buf.clone()),
            TensorStorage::Heap(buf) => TensorStorage::Heap(buf.clone()),
        }
    }
}

impl<D: Dims, T, L: Layout> Tensor<Q0, D, T, L> {
    /// Create a proof-only tensor (zero-size, compile-time only)
    pub fn proof(shape: ConcreteShape) -> Self {
        assert_eq!(shape.num_elements(), 0, "Q0 tensor must have zero elements");
        Self {
            data: TensorStorage::Zero(PhantomData),
            shape,
            layout: L::default(),
            _dims: PhantomData,
        }
    }

    /// Get the shape
    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    /// Get the rank
    pub fn rank(&self) -> usize {
        self.shape.rank()
    }
}

impl<D: Dims, T, L: Layout> Tensor<Q1, D, T, L> {
    /// Create a linear tensor with unique ownership
    pub fn linear(data: Box<[T]>, shape: ConcreteShape) -> Self {
        assert_eq!(data.len(), shape.num_elements(), "Data length must match shape");
        Self {
            data: TensorStorage::Linear(data),
            shape,
            layout: L::default(),
            _dims: PhantomData,
        }
    }

    /// Create a linear tensor from a vector
    pub fn from_vec(vec: Vec<T>, shape: ConcreteShape) -> Self {
        Self::linear(vec.into_boxed_slice(), shape)
    }

    /// Create an uninitialized linear tensor
    pub fn uninitialized(shape: ConcreteShape) -> Self
    where
        T: Default,
    {
        let len = shape.num_elements();
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::default());
        }
        Self::from_vec(vec, shape)
    }

    /// Get the shape
    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    /// Get the rank
    pub fn rank(&self) -> usize {
        self.shape.rank()
    }

    /// Get immutable slice access
    pub fn as_slice(&self) -> &[T] {
        match &self.data {
            TensorStorage::Linear(buf) => buf.as_ref(),
            _ => unreachable!(),
        }
    }

    /// Get mutable slice access (linear, unique)
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.data {
            TensorStorage::Linear(buf) => buf.as_mut(),
            _ => unreachable!(),
        }
    }

    /// Consume and return the underlying buffer
    pub fn into_inner(self) -> Box<[T]> {
        match self.data {
            TensorStorage::Linear(buf) => buf,
            _ => unreachable!(),
        }
    }

    /// Reshape the tensor (preserves linear ownership)
    pub fn reshape<D2: Dims>(self, new_shape: ConcreteShape) -> Tensor<Q1, D2, T, L> {
        assert_eq!(self.shape.num_elements(), new_shape.num_elements());
        Tensor {
            data: self.data,
            shape: new_shape,
            layout: self.layout,
            _dims: PhantomData,
        }
    }

    /// Transpose last two dimensions (for matrices)
    pub fn transpose(mut self) -> Self
    where
        D: Dims,
    {
        let mut dims = self.shape.dims().to_vec();
        let len = dims.len();
        if len >= 2 {
            dims.swap(len - 2, len - 1);
        }
        let new_shape = ConcreteShape::new(dims);
        self.shape = new_shape;
        self
    }
}

impl<D: Dims, T, L: Layout> Tensor<QStar, D, T, L> {
    /// Create a heap tensor with shared ownership
    pub fn heap(data: std::sync::Arc<[T]>, shape: ConcreteShape) -> Self {
        assert_eq!(data.len(), shape.num_elements(), "Data length must match shape");
        Self {
            data: TensorStorage::Heap(data),
            shape,
            layout: L::default(),
            _dims: PhantomData,
        }
    }

    /// Create a heap tensor from a vector
    pub fn from_vec(vec: Vec<T>, shape: ConcreteShape) -> Self {
        Self::heap(vec.into_boxed_slice().into(), shape)
    }

    /// Create a heap tensor by cloning a linear tensor
    pub fn from_linear(linear: Tensor<Q1, D, T, L>) -> Self {
        let data = linear.into_inner();
        Self::heap(data.into(), linear.shape)
    }

    /// Get the shape
    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    /// Get the rank
    pub fn rank(&self) -> usize {
        self.shape.rank()
    }

    /// Get immutable slice access (shared)
    pub fn as_slice(&self) -> &[T] {
        match &self.data {
            TensorStorage::Heap(buf) => buf.as_ref(),
            _ => unreachable!(),
        }
    }

    /// Try to get mutable access (fails if shared)
    pub fn try_mut_slice(&mut self) -> Option<&mut [T]> {
        match &mut self.data {
            TensorStorage::Heap(buf) => std::sync::Arc::get_mut(buf).map(|b| b.as_mut()),
            _ => None,
        }
    }

    /// Reshape the tensor (preserves heap sharing)
    pub fn reshape<D2: Dims>(self, new_shape: ConcreteShape) -> Tensor<QStar, D2, T, L> {
        assert_eq!(self.shape.num_elements(), new_shape.num_elements());
        Tensor {
            data: self.data,
            shape: new_shape,
            layout: self.layout,
            _dims: PhantomData,
        }
    }

    /// Transpose last two dimensions (for matrices)
    pub fn transpose(self) -> Self {
        let mut dims = self.shape.dims().to_vec();
        let len = dims.len();
        if len >= 2 {
            dims.swap(len - 2, len - 1);
        }
        let new_shape = ConcreteShape::new(dims);
        Tensor {
            data: self.data,
            shape: new_shape,
            layout: self.layout,
            _dims: PhantomData,
        }
    }
}

// Common methods for all quantities
impl<Q: QttQty, D: Dims, T, L: Layout> Tensor<Q, D, T, L> {
    /// Get the shape
    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    /// Get the rank
    pub fn rank(&self) -> usize {
        self.shape.rank()
    }

    /// Get number of elements
    pub fn num_elements(&self) -> usize {
        self.shape.num_elements()
    }

    /// Get the layout
    pub fn layout(&self) -> L {
        self.layout
    }

    /// Check if tensor is empty
    pub fn is_empty(&self) -> bool {
        self.num_elements() == 0
    }
}

// Q1-specific: unique mutable access
impl<D: Dims, T, L: Layout> Deref for Tensor<Q1, D, T, L> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<D: Dims, T, L: Layout> DerefMut for Tensor<Q1, D, T, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

// QStar-specific: shared immutable access
impl<D: Dims, T, L: Layout> Deref for Tensor<QStar, D, T, L> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

// Indexing support
impl<Q: QttQty, D: Dims, T, L: Layout> std::ops::Index<usize> for Tensor<Q, D, T, L>
where
    Tensor<Q, D, T, L>: Deref<Target = [T]>,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.deref()[index]
    }
}

impl<D: Dims, T, L: Layout> std::ops::IndexMut<usize> for Tensor<Q1, D, T, L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.deref_mut()[index]
    }
}

// Conversion traits
impl<D: Dims, T: Clone, L: Layout> From<Tensor<Q1, D, T, L>> for Tensor<QStar, D, T, L> {
    fn from(linear: Tensor<Q1, D, T, L>) -> Self {
        Tensor::from_linear(linear)
    }
}

impl<D: Dims, T: Clone, L: Layout> From<Tensor<Q0, D, T, L>> for Tensor<Q1, D, T, L>
where
    T: Default,
{
    fn from(_proof: Tensor<Q0, D, T, L>) -> Self {
        let shape = _proof.shape().clone();
        Tensor::uninitialized(shape)
    }
}

impl<D: Dims, T: Clone, L: Layout> From<Tensor<Q0, D, T, L>> for Tensor<QStar, D, T, L>
where
    T: Default,
{
    fn from(_proof: Tensor<Q0, D, T, L>) -> Self {
        let shape = _proof.shape().clone();
        let len = shape.num_elements();
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::default());
        }
        Tensor::from_vec(vec, shape)
    }
}

// Display for debugging
impl<Q: QttQty, D: Dims, T: std::fmt::Debug, L: Layout> std::fmt::Debug for Tensor<Q, D, T, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tensor")
            .field("qty", &Q::QTY)
            .field("shape", &self.shape)
            .field("layout", &std::any::type_name::<L>())
            .field("data", &self.as_slice())
            .finish()
    }
}

// Default layout
impl Default for RowMajor {
    fn default() -> Self {
        RowMajor
    }
}

impl Default for ColMajor {
    fn default() -> Self {
        ColMajor
    }
}

/// Tensor view for borrowing without ownership transfer
pub struct TensorView<'a, T, L: Layout = RowMajor> {
    data: &'a [T],
    shape: ConcreteShape,
    layout: L,
}

impl<'a, T, L: Layout> TensorView<'a, T, L> {
    pub fn new(data: &'a [T], shape: ConcreteShape) -> Self {
        assert_eq!(data.len(), shape.num_elements());
        Self {
            data,
            shape,
            layout: L::default(),
        }
    }

    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    pub fn as_slice(&self) -> &[T] {
        self.data
    }
}

impl<'a, T, L: Layout> Deref for TensorView<'a, T, L> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

/// Mutable tensor view for unique mutable borrowing
pub struct TensorViewMut<'a, T, L: Layout = RowMajor> {
    data: &'a mut [T],
    shape: ConcreteShape,
    layout: L,
}

impl<'a, T, L: Layout> TensorViewMut<'a, T, L> {
    pub fn new(data: &'a mut [T], shape: ConcreteShape) -> Self {
        assert_eq!(data.len(), shape.num_elements());
        Self {
            data,
            shape,
            layout: L::default(),
        }
    }

    pub fn shape(&self) -> &ConcreteShape {
        &self.shape
    }

    pub fn as_slice(&self) -> &[T] {
        self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.data
    }
}

impl<'a, T, L: Layout> Deref for TensorViewMut<'a, T, L> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<'a, T, L: Layout> DerefMut for TensorViewMut<'a, T, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}