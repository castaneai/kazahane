# kazahane packets

- little endian

```
[version info] (14 bytes) // "KAZAHANE 1.0.0" ASCII
[packet_type] (uint8)     // packet type
[payload_size] (uint16)   // payload size
[payload] (bytes[payload_size])
```

## Packet Types

### hello request (0x01)

```
[JSON Web Token] (variable length)
```

### hello response (0x02)

```
[status_code] (uint8)
[message_length] (uint8)
[message] (bytes[message_length])
```

### join room (0x03)

```
[JSON Web Token] (variable length)
```
