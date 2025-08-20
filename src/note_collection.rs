pub trait NoteCollection {
	fn new() -> Self;
	fn is_empty(&self) -> bool;
}

pub mod default {
	use super::NoteCollection;
	pub type DefaultNote = [u8; 2];

	#[derive(Clone)]
	pub struct DefaultNoteCollectionWith<N = DefaultNote>(heapless::Vec<N, 128>);

	impl<N> NoteCollection for DefaultNoteCollectionWith<N> {
		fn is_empty(&self) -> bool {
			self.0.is_empty()
		}
		fn new() -> Self {
			Self(heapless::Vec::<N, 128>::new())
		}
	}
	pub type DefaultNoteCollection = DefaultNoteCollectionWith<DefaultNote>;
}
