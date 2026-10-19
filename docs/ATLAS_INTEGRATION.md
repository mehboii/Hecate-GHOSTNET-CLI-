# Hecate and Atlas integration

Hecate embeds a Node bridge that imports `@n11x/atlas`. The verified package API is Atlas 0.2.0. Hecate and Atlas each connect directly to the configured GhostNet WebSocket relay; there is no daemon or local IPC service.

## API mapping

| Hecate operation | Atlas API |
| --- | --- |
| `identity create` | `new Atlas().createIdentity()` |
| `identity load` | `new Atlas().loadIdentity(GHOSTNET_SEED)` |
| `peer add` | `new Atlas().addPeer(publicKey)` and compare the derived `nodeId` |
| `send` | Load identity, call `addPeer(publicKey)`, verify its node ID, `connect()`, `send()`, then `disconnect()` |
| `listen` | Load identity, subscribe to events, then `connect()` |

Atlas keeps `requireEncryption` enabled by default. Hecate never disables it.

## Establishing trust

An Atlas node ID is derived from its Ed25519 public key. Obtain the intended peer's node ID and public key through an authenticated channel independent of the relay—for example, an in-person comparison or an already authenticated account. Then register both:

```sh
hecate peer add alice 0xEXPECTED_NODE_ID VERIFIED_LOWERCASE_PUBLIC_KEY
```

Hecate validates both formats, asks Atlas to derive the node ID from the public key, and persists the record only when the derived and expected IDs match. Configuration stores public material only. Hecate does not automatically trust or persist keys from relay announcements or incoming messages.

`hecate send alice "hello"` resolves the trusted record, passes its public key to Atlas, repeats the node-ID binding check in the bridge, and sends with Atlas's encryption-required default. An unknown label/node ID, malformed key, mismatch, SDK error, or failed WebSocket write fails the command.

## Delivery boundary

Successful `send()` completion confirms a WebSocket write. GhostNet has no delivery receipt. Only a recipient-side listener that receives and decrypts the message can establish end-to-end delivery.

For live testing, use two disposable identities, isolated `GHOSTNET_HOME` directories, and an authorized staging relay. Start the recipient listener first, register the recipient's verified public identity on the sender, send a unique test message, and confirm the recipient output. Do not disable TLS or Atlas encryption to make the test pass.
