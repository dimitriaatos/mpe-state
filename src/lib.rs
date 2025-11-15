//! # mpe-state - State management for MIDI Polyphonic Expression

// #![no_std]
use core::ops::Range;
use note_collection::{NoteCollection, default::DefaultNoteCollection};
pub mod note_collection;

pub enum Mode {
	Mode1,
	Mode2,
	// Poly Mode
	Mode3,
	// Mono Mode
	Mode4,
	Mode5,
}

#[derive(Clone, Copy)]
pub enum Zone {
	Lower,
	Upper,
}

impl Zone {
	pub fn new(manager_channel: u8) -> Option<Self> {
		match manager_channel {
			0 => Some(Self::Lower),
			15 => Some(Self::Upper),
			_ => None,
		}
	}
	fn get_other(&self) -> Self {
		match self {
			Self::Lower => Self::Upper,
			Self::Upper => Self::Lower,
		}
	}
	pub fn get_by_manager(&self, manager: u8) -> Option<Zone> {
		match manager {
			0 => Some(Self::Lower),
			15 => Some(Self::Upper),
			_ => None,
		}
	}
	pub fn manager_channel(&self) -> u8 {
		match self {
			Self::Lower => 0,
			Self::Upper => 15,
		}
	}
}

pub enum ChannelType {
	Manager,
	Member,
	Conventional,
}

#[derive(Clone, Debug)]
pub struct MIDIChannel<C = DefaultNoteCollection>
where
	C: NoteCollection,
{
	pitch_bend_sensitivity: u8,
	pub pitch_bend: f32,
	pub channel_pressure: f32,
	pub timbre_control: f32,
	pub notes: C,
}

impl<C> Default for MIDIChannel<C>
where
	C: NoteCollection,
{
	fn default() -> Self {
		Self {
			pitch_bend_sensitivity: 2,
			pitch_bend: 0.,
			channel_pressure: 0.,
			timbre_control: 0.,
			notes: C::new(),
		}
	}
}

impl<C> MIDIChannel<C>
where
	C: NoteCollection,
{
	pub fn new(channel_type: ChannelType) -> Self {
		match channel_type {
			ChannelType::Member => Self { pitch_bend_sensitivity: 48, ..Default::default() },
			_ => Self { ..Default::default() },
		}
	}
	pub fn pitch_bend_sensitivity(&self) -> u8 {
		self.pitch_bend_sensitivity
	}
}

#[derive(Clone, Debug)]
pub enum Channel<C = DefaultNoteCollection>
where
	C: NoteCollection,
{
	Manager { member_channels: u8, channel: MIDIChannel<C> },
	Member { channel: MIDIChannel<C> },
	Conventional { channel: MIDIChannel<C> },
}

impl<C> Channel<C>
where
	C: NoteCollection,
{
	pub fn new_member() -> Self {
		Self::Member { channel: MIDIChannel::<C>::new(ChannelType::Member) }
	}
	pub fn new_conventional() -> Self {
		Self::Conventional { channel: MIDIChannel::<C>::new(ChannelType::Conventional) }
	}
	pub fn new_manager(member_channels: u8) -> Self {
		Self::Manager { channel: MIDIChannel::<C>::new(ChannelType::Manager), member_channels }
	}
}

pub struct MPEState<C = DefaultNoteCollection>
where
	C: NoteCollection,
{
	pub channels: [Channel<C>; 16],
}

impl<C> Default for MPEState<C>
where
	C: NoteCollection,
{
	fn default() -> Self {
		Self { channels: core::array::from_fn(|_| Channel::new_conventional()) }
	}
}

impl<C> MPEState<C>
where
	C: NoteCollection + Clone,
{
	/// Creates a new instance of MPEState.
	/// MPE status is disabled and all channels are set to conventional.
	pub fn new() -> Self {
		Self { channels: core::array::from_fn(|_| Channel::<C>::new_conventional()) }
	}
	/// Configures the member channels of an MPE zone.
	/// Zero member channels disable the zone.
	pub fn config(&mut self, zone: Zone, member_channels: u8) {
		let manager_index = zone.manager_channel();

		let prev_member_channels: u8 = match self.channels[manager_index as usize] {
			Channel::Manager { member_channels, .. } => member_channels,
			_ => 0,
		};

		match member_channels {
			// If the new number of member channels is zero
			0 => {
				if let Channel::Manager { .. } = self.channels[manager_index as usize] {
					// and the zone was enabled, set the manager and all member channels to conventional.
					self.zone_channels_mut(zone).unwrap().fill(Channel::<C>::new_conventional());
				}
			},
			// If the new number of member channels is greater that previously,
			new_member_channels if new_member_channels > prev_member_channels => {
				match &mut self.channels[manager_index as usize] {
					// and the zone was enabled, increase the member_channels property,
					Channel::Manager { member_channels, .. } => {
						*member_channels = new_member_channels
					},
					// if the zone wasn't enabled, creating a manager channel.
					_ => {
						self.channels[zone.manager_channel() as usize] =
							Channel::new_manager(member_channels)
					},
				}
				// Initializing only the added member channels
				self.zone_slice_mut(
					zone,
					(prev_member_channels.max(1) as usize)..(new_member_channels as usize + 1),
				)
				.fill(Channel::new_member());
				// If another zone is enabled, checking if its channels are overlapping and decreasing them

				let zone_channels = self.zone_channels(zone).map_or(0, |c| c.len());
				let other_zone_channels =
					self.zone_channels(zone.get_other()).map_or(0, |c| c.len());
				if let Channel::Manager { member_channels, .. } =
					&mut self.channels[zone.get_other().manager_channel() as usize]
				{
					match 16 - zone_channels {
						1 => {
							self.channels[zone.get_other().manager_channel() as usize] =
								Channel::new_conventional();
						},
						remaining_channels if other_zone_channels > remaining_channels => {
							*member_channels = remaining_channels as u8 - 1;
						},
						_ => {},
					}
				}
			},
			// If the new number of member channels is less than previously,
			// (this means that the zone was already enabled with more member channels)
			new_member_channels if new_member_channels < prev_member_channels => {
				if let Channel::Manager { member_channels, .. } =
					&mut self.channels[manager_index as usize]
				{
					// Decreasing the member_channels property
					*member_channels = new_member_channels;
					// and convert all the removed member channels to conventional
					self.zone_slice_mut(
						zone,
						(new_member_channels as usize)..(prev_member_channels as usize + 1),
					)
					.fill(Channel::new_conventional());
				}
			},
			_ => {},
		};
	}
	/// Returns the status of MPE mode
	pub fn active(&self) -> bool {
		matches!(self.channels.first().unwrap(), Channel::Manager { .. })
			|| matches!(self.channels.last().unwrap(), Channel::Manager { .. })
	}

	// Zone methods

	/// Returns a range containing the indexes of a given zone's member channels.
	pub fn zone_member_channel_range(&self, zone: Zone) -> Option<Range<usize>> {
		match self.channels[zone.manager_channel() as usize] {
			Channel::Manager { member_channels, .. } => {
				let manager_offset = 1;
				Some(Self::compute_range(
					zone,
					manager_offset..(member_channels as usize + manager_offset),
				))
			},
			_ => None,
		}
	}
	/// Returns a slice containing the member channels of a given zone.
	pub fn zone_member_channels(&self, zone: Zone) -> Option<&[Channel<C>]> {
		self.zone_member_channel_range(zone).map(|range| &self.channels[range])
	}
	/// Returns a mutable slice containing the member channels of a given zone.
	pub fn zone_member_channels_mut(&mut self, zone: Zone) -> Option<&mut [Channel<C>]> {
		self.zone_member_channel_range(zone).map(|range| &mut self.channels[range])
	}
	/// Returns a range containing the indexes of all channels of a given zone.
	pub fn zone_channel_range(&self, zone: Zone) -> Option<Range<usize>> {
		match self.channels[zone.manager_channel() as usize] {
			Channel::Manager { member_channels, .. } => {
				let manager_offset = 1;
				Some(Self::compute_range(zone, 0..(member_channels as usize + manager_offset)))
			},
			_ => None,
		}
	}
	/// Returns a slice containing the all channels of a given zone.
	pub fn zone_channels(&self, zone: Zone) -> Option<&[Channel<C>]> {
		self.zone_channel_range(zone).map(|range| &self.channels[range])
	}
	/// Returns a mutable slice containing the all channels of a given zone.
	pub fn zone_channels_mut(&mut self, zone: Zone) -> Option<&mut [Channel<C>]> {
		self.zone_channel_range(zone).map(|range| &mut self.channels[range])
	}
	/// Inverts a range of channel indexes, allowing the upper zone to be zero indexed.
	fn compute_range(zone: Zone, range: Range<usize>) -> Range<usize> {
		let manager_index = zone.manager_channel();
		let start = range.start.abs_diff(manager_index as usize);
		let end = range.end.abs_diff(manager_index as usize);
		if matches!(zone, Zone::Lower) { start..end } else { (end + 1)..(start + 1) }
	}
	/// Returns a slice of channels, allowing the upper zone to be zero indexed.
	pub fn zone_slice(&self, zone: Zone, range: Range<usize>) -> &[Channel<C>] {
		&self.channels[Self::compute_range(zone, range)]
	}
	/// Returns a mutable slice of channels, allowing the upper zone to be zero indexed.
	pub fn zone_slice_mut(&mut self, zone: Zone, range: Range<usize>) -> &mut [Channel<C>] {
		&mut self.channels[Self::compute_range(zone, range)]
	}

	// channel methods
	/// Return the zone to which the given channel belongs, None if it doesn't belong to a zone.
	pub fn zone_by_channel(&self, channel: u8) -> Option<Zone> {
		[Zone::Lower, Zone::Upper]
			.iter()
			.find(|z| self.zone_channel_range(**z).is_some_and(|r| r.contains(&(channel as usize))))
			.copied()
	}
	/// Sets pitch bend sensitivity for a given channel, implementing MIDI 1.0 compatibility.
	/// The MPE specification requires MPE receivers to apply the same pitch bend sensitivity
	/// to all member channels when it is changed on a single channel.
	pub fn set_pitch_bend_sensitivity(&mut self, channel: u8, pitch_bend_sensitivity: u8) {
		let zone = self.zone_by_channel(channel);
		match &mut self.channels[channel as usize] {
			Channel::Manager { channel, .. } | Channel::Conventional { channel } => {
				channel.pitch_bend_sensitivity = pitch_bend_sensitivity.max(2);
			},
			Channel::Member { .. } => {
				// changing a single member channel's pitch bend sensitivity
				// should be reflected to all member channels of the zone
				self.zone_member_channels_mut(zone.unwrap()).unwrap().iter_mut().for_each(
					|channel| {
						if let Channel::Member { channel } = channel {
							channel.pitch_bend_sensitivity = pitch_bend_sensitivity;
						}
					},
				);
			},
		}
	}
	/// Returns the channel for the given channel index.
	pub fn get_channel(&self, channel: u8) -> Option<&MIDIChannel<C>> {
		self.channels.get(channel as usize).map(|c| match c {
			Channel::Conventional { channel }
			| Channel::Manager { channel, .. }
			| Channel::Member { channel } => channel,
		})
	}
	pub fn get_channel_mut(&mut self, channel: u8) -> &mut MIDIChannel<C> {
		match self.channels.get_mut(channel as usize).unwrap() {
			Channel::Conventional { channel }
			| Channel::Manager { channel, .. }
			| Channel::Member { channel } => channel,
		}
	}
	pub fn unoccupied_channel(&self, zone: Zone) -> Option<u8> {
		match self.zone_member_channel_range(zone) {
			None => None,
			Some(range) => {
				let mut r = range.into_iter();
				r.find(|&i| {
					let ch = i as u8;
					self.get_channel(ch).unwrap().notes.is_empty()
				})
				.map(|i| i as u8)
			},
		}
	}
}
