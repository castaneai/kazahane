use binrw::binrw;

#[binrw]
#[brw(little, magic = b"KAZAHANE 1.0.0")]
#[derive(Debug, PartialEq)]
pub enum Packet {
    #[brw(magic = 0x01u8)]
    HelloRequest {
        #[br(temp)]
        #[bw(calc = token.len() as u16)]
        token_size: u16,
        #[br(count = token_size)]
        token: Vec<u8>,
    },

    #[brw(magic = 0x02u8)]
    HelloResponse {
        status_code: HelloResponseStatusCode,
        #[br(temp)]
        #[bw(calc = message.len() as u16)]
        message_size: u16,
        #[br(count = message_size)]
        message: Vec<u8>,
    },

    #[brw(magic = 0x03u8)]
    JoinRoomRequest { room_id: uuid::Bytes },

    #[brw(magic = 0x04u8)]
    JoinRoomResponse {},

    #[brw(magic = 0x05u8)]
    BroadcastRequest {
        #[br(temp)]
        #[bw(calc = payload.len() as u16)]
        payload_size: u16,
        #[br(count = payload_size)]
        payload: Vec<u8>,
    },

    #[brw(magic = 0x06u8)]
    RoomNotification(RoomNotification),

    #[brw(magic = 0x07u8)]
    ServerNotification(ServerNotification),

    #[brw(magic = 0xDEu8)]
    TestCountUp {},

    #[brw(magic = 0xDFu8)]
    TestCountUpResponse { counter: u64 },
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub enum RoomNotification {
    #[brw(magic = 0x01u8)]
    PlayerJoined { player: uuid::Bytes },

    #[brw(magic = 0x02u8)]
    PlayerLeft { player: uuid::Bytes },

    #[brw(magic = 0x03u8)]
    Broadcast {
        #[br(temp)]
        #[bw(calc = payload.len() as u16)]
        payload_size: u16,
        #[br(count = payload_size)]
        payload: Vec<u8>,
    },
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq)]
pub enum ServerNotification {
    #[brw(magic = 0x01u8)]
    Shutdown,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum HelloResponseStatusCode {
    Unknown = 0x00,
    OK = 0x01,
    Denied = 0x02,
}

#[cfg(test)]
mod tests {
    use crate::packets::Packet;
    use binrw::io::Cursor;
    use binrw::{BinReaderExt, BinWrite};

    #[test]
    fn read_packet() {
        let p: Packet = Cursor::new(b"KAZAHANE 1.0.0\x01\x05\x00hello")
            .read_le()
            .unwrap();
        assert_eq!(
            p,
            Packet::HelloRequest {
                token: b"hello".to_vec()
            }
        );
    }

    #[test]
    fn write_packet() {
        let mut writer = Cursor::new(Vec::new());
        let p = Packet::HelloRequest {
            token: b"world".to_vec(),
        };
        p.write_to(&mut writer).unwrap();
        assert_eq!(&writer.into_inner()[..], b"KAZAHANE 1.0.0\x01\x05\x00world");
    }
}
