# kazahane packets

- little endian

```
[version info] (14 bytes) // "KAZAHANE 1.0.0" ASCII
[packet_type] (uint8)     // packet type
[payload_size] (uint16)   // payload size
[payload] (bytes[payload_size])
```

## hello request (packet_type: 0x01)

### Payload

```
[JSON Web Token] (variable length)
```

## hello response (packet_type: 0x02)

### Payload

```
[status_code] (uint8)
[message_length] (uint8)
[message] (bytes[message_length])
```

### Status code:

- 0x00: Unknown
- 0x01: OK
- 0x02: Denied
