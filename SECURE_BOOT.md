# Secure Boot — STM32F4 + X-CUBE-SBSFU

**Status:** Integration guide ready. Not yet implemented (requires STM32F4 hardware).

This document is the complete integration guide for adding a hardware
root of trust to RNFC using STMicroelectronics X-CUBE-SBSFU. It closes
threat T-09 (firmware replacement) and requirement SEC-10.

## 1. Why Secure Boot

Without Secure Boot, an attacker with physical access to the drone can:

1. Connect to the STM32 debug port (SWD)
2. Erase the existing firmware
3. Flash a malicious firmware that looks like RNFC
4. The drone flies, but with attacker-controlled logic

Secure Boot prevents this by verifying the firmware signature before
execution. The verification key is stored in protected flash that cannot
be read or modified.

## 2. Architecture

```
+------------------+  0x08000000
| SBSFU Bootloader |  (protected: RDP level 1, WRP)
|  - Root of Trust |
|  - Verifies sig  |
+------------------+  0x08020000
| Active Slot      |
|  RNFC firmware   |  (Rust, signed with ECDSA P-256)
|  (slot 0)        |
+------------------+  0x08040000
| Download Slot    |
|  (slot 1)        |  (for over-the-air update)
+------------------+  0x08060000
| Swap Region      |
|  (SFU scratch)   |
+------------------+  0x08080000
```

The bootloader occupies the first 128 KB. The application slots are
128 KB each. The layout is configurable in `linker.icf` / `.ld`.

## 3. Memory Layout (STM32F407, 1 MB Flash)

| Region | Start | Size | Purpose |
|---|---|---|---|
| SBSFU | 0x08000000 | 64 KB | Bootloader + keys |
| Active slot | 0x08010000 | 192 KB | RNFC firmware |
| Download slot | 0x08040000 | 192 KB | OTA staging |
| Swap region | 0x08070000 | 64 KB | SFU scratch |
| NVM / config | 0x08080000 | 512 KB | CertChain, logs |

This layout is a starting point. Adjust to your board and application size.

## 4. Prerequisites

| Requirement | How to obtain |
|---|---|
| STM32F4 board | STM32F4-Discovery (~€15) or Pixhawk with F4 |
| ST-LINK V2/V3 | For flashing and debugging (~€10) |
| STM32CubeIDE | Free from ST (includes X-CUBE-SBSFU) |
| OpenSSL | For key generation and signing |
| Python 3 | For the image preparation script |

## 5. Integration Steps

### Step 1 — Obtain X-CUBE-SBSFU

1. Open STM32CubeIDE → Help → Manage embedded software packages
2. Install `X-CUBE-SBSFU` (STMicroelectronics)
3. Or download from https://www.st.com/en/embedded-software/x-cube-sbsfu.html
4. Read AN5056 (Getting started with X-CUBE-SBSFU) — the official guide

### Step 2 — Generate Keys

```bash
# Generate ECDSA P-256 key pair
openssl ecparam -name prime256v1 -genkey -noout -out security/secure_boot/private.pem
openssl ec -in security/secure_boot/private.pem -pubout -out security/secure_boot/public.pem

# The PUBLIC key goes into the SBSFU binary (compiled in)
# The PRIVATE key stays offline, used only to sign firmware
```

**CRITICAL:** The private key must never be committed to git. It is in
`.gitignore` (`security/secure_boot/*.pem`).

### Step 3 — Build the SBSFU Bootloader

1. In STM32CubeIDE, open the SBSFU example for your board
   (e.g., `STM32F4-Discovery/SBSFU_Boot`)
2. Configure `SECoreBin/Config/` with your memory layout (section 3)
3. Replace the public key with yours (generated in Step 2)
4. Build → produces `SBSFU_Boot.bin`
5. Flash at 0x08000000

### Step 4 — Sign the RNFC Firmware

The RNFC firmware is built by `cargo build --target thumbv7em-none-eabihf --release`.
The resulting binary must be signed before flashing.

```powershell
# Build the Rust firmware
cargo build -p rnfc-hal --target thumbv7em-none-eabihf --release

# Sign it (script provided in security/secure_boot/)
.\\security\\secure_boot\\sign_firmware.ps1 `
    -Input  target/thumbv7em-none-eabihf/release/rnfc-hal `
    -Output build/rnfc-signed.bin `
    -Key    security/secure_boot/private.pem
```

### Step 5 — Flash and Verify

```bash
# Flash bootloader
STM32_Programmer_CLI -c port=SWD -w SBSFU_Boot.bin 0x08000000

# Flash signed firmware
STM32_Programmer_CLI -c port=SWD -w rnfc-signed.bin 0x08010000

# Protect flash (RDP level 1, WRP)
STM32_Programmer_CLI -c port=SWD -ob RDP=0xBB
```

After flashing, the bootloader will:
1. Read the signature from the firmware header
2. Verify with the stored public key
3. If valid: jump to application
4. If invalid: block boot, LED indicates failure

### Step 6 — Test the Attack

```bash
# Generate a malicious firmware (different content)
echo "malicious" > malicious.bin

# Flash it (replaces the signed firmware)
STM32_Programmer_CLI -c port=SWD -w malicious.bin 0x08010000

# Reset the board
# Expected: bootloader blocks, LED stays off, no RNFC output
```

If the malicious firmware runs, Secure Boot is not working.

## 6. What This Closes

| Threat | Before | After |
|---|---|---|
| T-09 Firmware replacement | ❌ Vulnerable | ✅ Blocked |
| SEC-10 Firmware verified | ❌ Not implemented | ✅ Implemented |

## 7. What This Does NOT Close

| Gap | Why |
|---|---|
| Supply chain (SEC-13) | Secure Boot verifies firmware, not dependencies |
| Insider (SEC-12) | An operator with the private key can sign malicious firmware |
| Side channels | Power/timing analysis not addressed |
| Debug port (SWD) | Must be disabled after flashing (RDP level 2) |

## 8. Operational Considerations

### Key Management

- The private key is the **crown jewel**. If compromised, all drones with
  the corresponding public key are vulnerable.
- Store in HSM (Hardware Security Module) or air-gapped machine.
- Rotate keys annually. Rotation requires re-flashing the bootloader.

### Firmware Update Process

The SFU (Secure Firmware Update) allows OTA updates:

1. Operator signs new firmware with private key
2. Uploads via ground station (Ymodem over UART)
3. Bootloader receives into Download slot
4. Bootloader verifies signature
5. If valid: swaps slots, reboots into new firmware
6. If invalid: rejects, keeps current firmware

This requires a companion-side uploader (not yet implemented).

### RDP Level 2

For production, set RDP level 2 (irreversible):

```bash
STM32_Programmer_CLI -c port=SWD -ob RDP=0xCC
```

**WARNING:** RDP level 2 disables the debug port permanently. Only do
this on production units, never on development boards.

## 9. Cost and Effort

| Item | Cost | Effort |
|---|---|---|
| STM32F4-Discovery board | ~€15 | — |
| ST-LINK V2 | ~€10 | — |
| Integration (read AN5056, configure, build) | — | 3-5 days |
| Key management procedure | — | 1 day |
| Testing (valid + malicious firmware) | — | 1 day |
| **Total** | **~€25** | **1-2 weeks** |

## 10. Status

| Component | Status |
|---|---|
| Integration guide (this document) | ✅ Complete |
| Signing script | ✅ Complete (`sign_firmware.ps1`) |
| Key generation script | ✅ Complete (`gen_keys.ps1`) |
| SBSFU bootloader build | ⏳ Requires hardware |
| Rust firmware signing | ⏳ Requires hardware |
| End-to-end test | ⏳ Requires hardware |

**When the hardware arrives, follow sections 5.1 to 5.6.**

---

*Last updated: v6.1.0*
*Reference: AN5056 (STMicroelectronics), DO-326A §6.2*
