#![no_std]
use core::ops::Range;

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
	pub fn new(manager_channel: usize) -> Option<Self> {
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
	pub fn get_by_manager(&self, manager: usize) -> Option<Zone> {
		match manager {
			0 => Some(Self::Lower),
			15 => Some(Self::Upper),
			_ => None,
		}
	}
	pub fn manager_channel(&self) -> usize {
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
pub struct MIDIChannel {
	pitch_bend_sensitivity: u8,
	pub pitch_bend: f32,
	pub channel_pressure: f32,
	pub timbre_control: f32,
	pub notes: heapless::Vec<[u8; 128], 128>,
}

impl Default for MIDIChannel {
	fn default() -> Self {
		Self {
			pitch_bend_sensitivity: 2,
			pitch_bend: 0.,
			channel_pressure: 0.,
			timbre_control: 0.,
			notes: heapless::Vec::new(),
		}
	}
}

impl MIDIChannel {
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
pub enum Channel {
	Manager { member_channels: u8, channel: MIDIChannel },
	Member { channel: MIDIChannel },
	Conventional { channel: MIDIChannel },
}

impl Channel {
	pub fn new_member() -> Self {
		Self::Member { channel: MIDIChannel::new(ChannelType::Member) }
	}
	pub fn new_conventional() -> Self {
		Self::Conventional { channel: MIDIChannel::new(ChannelType::Conventional) }
	}
	pub fn new_manager(member_channels: u8) -> Self {
		Self::Manager { channel: MIDIChannel::new(ChannelType::Manager), member_channels }
	}
}

pub struct MPEState {
	pub channels: [Channel; 16],
}

impl Default for MPEState {
	fn default() -> Self {
		Self { channels: core::array::from_fn(|_| Channel::new_conventional()) }
	}
}

impl MPEState {
	pub fn new() -> Self {
		Self { channels: core::array::from_fn(|_| Channel::new_conventional()) }
	}
	pub fn config(&mut self, zone: &Zone, member_channels: u8) {
		let manager_index = zone.manager_channel();

		let prev_member_channels: u8 = match self.channels[manager_index] {
			Channel::Manager { member_channels, .. } => member_channels,
			_ => 0,
		};

		match member_channels {
			// If the new number of member channels is zero
			0 => match self.channels[manager_index] {
				// and the zone was enabled, set the manager and all member channels to conventional.
				Channel::Manager { .. } => {
					self.zone_channels_mut(zone).unwrap().fill(Channel::new_conventional());
				},
				_ => {},
			},
			// If the new number of member channels is greater that previously,
			new_member_channels if new_member_channels > prev_member_channels => {
				match &mut self.channels[manager_index] {
					// and the zone was enabled, increase the member_channels property,
					Channel::Manager { member_channels, .. } => {
						*member_channels = new_member_channels as u8
					},
					// if the zone wasn't enabled, creating a manager channel.
					_ => {
						self.channels[zone.manager_channel()] =
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
					self.zone_channels(&zone.get_other()).map_or(0, |c| c.len());
				if let Channel::Manager { member_channels, .. } =
					&mut self.channels[zone.get_other().manager_channel()]
				{
					match 16 - zone_channels {
						1 => {
							self.channels[zone.get_other().manager_channel()] =
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
				if let Channel::Manager { member_channels, .. } = &mut self.channels[manager_index]
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
	pub fn active(&self) -> bool {
		matches!(self.channels.first().unwrap(), Channel::Manager { .. })
			|| matches!(self.channels.last().unwrap(), Channel::Manager { .. })
	}

	// Zone methods
	pub fn zone_member_channel_range(&self, zone: &Zone) -> Option<Range<usize>> {
		match self.channels[zone.manager_channel()] {
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
	pub fn zone_member_channels(&self, zone: &Zone) -> Option<&[Channel]> {
		self.zone_member_channel_range(zone).map_or(None, |range| Some(&self.channels[range]))
	}
	pub fn zone_member_channels_mut(&mut self, zone: &Zone) -> Option<&mut [Channel]> {
		self.zone_member_channel_range(zone).map_or(None, |range| Some(&mut self.channels[range]))
	}
	pub fn zone_channel_range(&self, zone: &Zone) -> Option<Range<usize>> {
		match self.channels[zone.manager_channel()] {
			Channel::Manager { member_channels, .. } => {
				let manager_offset = 1;
				Some(Self::compute_range(zone, 0..(member_channels as usize + manager_offset)))
			},
			_ => None,
		}
	}
	pub fn zone_channels(&self, zone: &Zone) -> Option<&[Channel]> {
		self.zone_channel_range(zone).map_or(None, |range| Some(&self.channels[range]))
	}
	pub fn zone_channels_mut(&mut self, zone: &Zone) -> Option<&mut [Channel]> {
		self.zone_channel_range(zone).map_or(None, |range| Some(&mut self.channels[range]))
	}
	fn compute_range(zone: &Zone, range: Range<usize>) -> Range<usize> {
		let manager_index = zone.manager_channel();
		let start = range.start.abs_diff(manager_index);
		let end = range.end.abs_diff(manager_index);
		if matches!(zone, Zone::Lower) { start..end } else { (end + 1)..(start + 1) }
	}
	pub fn zone_slice(&self, zone: &Zone, range: Range<usize>) -> &[Channel] {
		&self.channels[Self::compute_range(zone, range)]
	}
	pub fn zone_slice_mut(&mut self, zone: &Zone, range: Range<usize>) -> &mut [Channel] {
		&mut self.channels[Self::compute_range(zone, range)]
	}

	// channel methods
	fn zone_by_channel(&self, channel: &usize) -> Option<Zone> {
		[Zone::Lower, Zone::Upper]
			.iter()
			.find(|z| self.zone_channel_range(&z).map_or(false, |r| r.contains(&channel)))
			.copied()
	}
	pub fn set_pitch_bend_sensitivity(&mut self, channel: usize, pitch_bend_sensitivity: u8) {
		let zone = self.zone_by_channel(&channel);
		match &mut self.channels[channel] {
			Channel::Manager { channel, .. } | Channel::Conventional { channel } => {
				channel.pitch_bend_sensitivity = pitch_bend_sensitivity;
			},
			Channel::Member { .. } => {
				// changing a single member channel's pitch bend sensitivity
				// should be reflected to all member channels of the zone
				self.zone_member_channels_mut(&zone.unwrap()).unwrap().iter_mut().for_each(
					|channel| {
						if let Channel::Member { channel } = channel {
							channel.pitch_bend_sensitivity = pitch_bend_sensitivity;
						}
					},
				);
			},
		}
	}
	pub fn get_channel(&self, channel: usize) -> Option<&MIDIChannel> {
		self.channels.get(channel).map(|c| match c {
			Channel::Conventional { channel }
			| Channel::Manager { channel, .. }
			| Channel::Member { channel } => channel,
		})
	}
	// pub fn add_note(&mut self, voice_allocation:){
	// self.channels
	// }
}
