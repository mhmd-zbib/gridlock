use super::MoveSpeed;

/// Packet sent from client → server every input tick (~11 bytes on the wire).
///
/// Wire layout (big-endian):
///   [sequence: u16] [timestamp: u32] [movement_x: i8] [movement_y: i8]
///   [rotation: u16] [flags: u8]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClientPacket {
    /// Monotonically increasing per-client sequence number; wraps at `u16::MAX`.
    /// Used server-side to discard out-of-order / duplicate inputs.
    pub sequence: u16,

    /// Client-local time in milliseconds. Echoed back in `ServerPacket::timestamp`
    /// so the client can measure round-trip latency.
    pub timestamp: u32,

    /// Horizontal movement intent: `-1` (left) · `0` (still) · `1` (right).
    pub movement_x: i8,

    /// Vertical movement intent: `-1` (up) · `0` (still) · `1` (down).
    pub movement_y: i8,

    /// Aim direction compressed to a full circle in `[0, 65535]`.
    /// Maps linearly to `[0, 2π)` radians; use [`encode_rotation`] /
    /// [`decode_rotation`] in the codec module for conversion.
    pub rotation: u16,

    /// Packed action bits; see [`InputFlags`].
    pub flags: InputFlags,
}

// ── InputFlags ───────────────────────────────────────────────────────────────

/// Bitfield carried inside [`ClientPacket::flags`].
///
/// ```text
/// bit 0   : SHOOTING
/// bit 1   : RELOADING
/// bits 2-3: MOVE_SPEED  (0 = SlowWalk · 1 = Walk · 2 = Run · 3 = reserved)
/// bit 4   : PEEKING
/// bits 5-7: weapon slot index (0-7)
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InputFlags(pub u8);

impl InputFlags {
    const SHOOTING: u8 = 1 << 0;
    const RELOADING: u8 = 1 << 1;
    const MOVE_SPEED_MASK: u8 = 0b0000_1100;
    const MOVE_SPEED_SHIFT: u8 = 2;
    const PEEKING: u8 = 1 << 4;
    const WEAPON_SLOT_MASK: u8 = 0b1110_0000;
    const WEAPON_SLOT_SHIFT: u8 = 5;

    // ── Getters ──────────────────────────────────────────────────────────────

    pub fn is_shooting(self) -> bool {
        self.0 & Self::SHOOTING != 0
    }
    pub fn is_reloading(self) -> bool {
        self.0 & Self::RELOADING != 0
    }
    /// Returns the active movement speed tier.
    pub fn move_speed(self) -> MoveSpeed {
        MoveSpeed::from_u8((self.0 & Self::MOVE_SPEED_MASK) >> Self::MOVE_SPEED_SHIFT)
    }
    pub fn is_peeking(self) -> bool {
        self.0 & Self::PEEKING != 0
    }
    /// Returns the active weapon slot index in `[0, 7]`.
    pub fn weapon_slot(self) -> u8 {
        (self.0 & Self::WEAPON_SLOT_MASK) >> Self::WEAPON_SLOT_SHIFT
    }

    // ── Setters ──────────────────────────────────────────────────────────────

    pub fn set_shooting(&mut self, v: bool) {
        self.set_bit(Self::SHOOTING, v);
    }
    pub fn set_reloading(&mut self, v: bool) {
        self.set_bit(Self::RELOADING, v);
    }
    /// Encodes the movement speed tier into bits 2-3.
    pub fn set_move_speed(&mut self, speed: MoveSpeed) {
        let bits = ((speed as u8) << Self::MOVE_SPEED_SHIFT) & Self::MOVE_SPEED_MASK;
        self.0 = (self.0 & !Self::MOVE_SPEED_MASK) | bits;
    }
    pub fn set_peeking(&mut self, v: bool) {
        self.set_bit(Self::PEEKING, v);
    }
    /// Clamps `slot` to `[0, 7]`.
    pub fn set_weapon_slot(&mut self, slot: u8) {
        let bits = (slot.min(7) << Self::WEAPON_SLOT_SHIFT) & Self::WEAPON_SLOT_MASK;
        self.0 = (self.0 & !Self::WEAPON_SLOT_MASK) | bits;
    }

    fn set_bit(&mut self, bit: u8, v: bool) {
        if v {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }
}
