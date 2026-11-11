# CERTIS Safety Case — Mission demo_2026_001

**Root hash (tamper-evident):** `0x2e49cd3d6c0dfa6b`

---

## 1. GSN (Goal Structuring Notation)

# GSN — Mission demo_2026_001

- **Goal G1**: RNFC flight controller is safe during mission
  - *Strategy S1*: Argument over all fault classes: sensor, actuator, pilot
- **Goal G1.1**: Sensor faults are mitigated by FDIR
    - Evidence Sn1.1: SensorHealth + SensorVoter detected 9 events in runtime
- **Goal G1.2**: Actuator faults are recovered by LNN-FTC
    - Evidence Sn1.2: LNN-FTC recovery Succeeded
- **Goal G1.3**: Pilot authority is preserved with safety envelope
    - Evidence Sn1.3: HASE safety filter + haptic feedback logged

---

## 2. FMEA (Failure Mode and Effects Analysis)

# FMEA — Mission demo_2026_001

| Failure Mode | Cause | Sev | Occ | Det | RPN | Level | Mitigation | Evidence |
|---|---|---|---|---|---|---|---|---|
| Single sensor stuck | Hardware failure in IMU/Mag | 7 | 2 | 1 | 14 | LOW | SensorHealth detects stuck, ZombieDetector -> RTL | 0 |
| Sensor spoofing | External magnetic interference or adversarial attack | 8 | 3 | 1 | 24 | LOW | SpoofingDetector (mag vs gyro divergence) | 1 |
| Sensor outlier | EMI, vibration, or calibration drift | 5 | 5 | 2 | 50 | MEDIUM | SensorVoter 2-of-3 majority voting | 1 |
| Motor loss (actuator failure) | Motor burn-out, ESC failure, or prop loss | 10 | 1 | 3 | 30 | LOW | LNN-FTC two-phase recovery (hover + landing) | 1 |
| Zombie orbit | Adversarial wind + sensor noise causes stuck | 6 | 4 | 2 | 48 | LOW | ZombieDetector (>60s stuck -> RTL latch) | 1 |
| GNSS dropout | Jamming, urban canyon, or tunnel | 7 | 4 | 1 | 28 | LOW | Kalman dead-reckoning 10s | 0 |
| Battery depletion | Long mission or unexpected drain | 7 | 3 | 2 | 42 | LOW | Failsafe RTL at 10.5V | 0 |
| Geofence breach | Pilot error or wind drift | 6 | 2 | 1 | 12 | LOW | Failsafe ReturnToLaunch | 0 |
| Pilot override unsafe command | Human error | 6 | 3 | 2 | 36 | LOW | HASE safety filter + haptic feedback | 1 |

---

## 3. Fault Tree Analysis (FTA)

```json
{
  "id": "TOP",
  "description": "Mission demo_2026_001 safety events",
  "event_count": 9,
  "children": [
    {
      "id": "failsafe_returntolaunch",
      "description": "Failsafe ReturnToLaunch triggered",
      "event_count": 2,
      "mitigation": "Failsafe layer",
    },
    {
      "id": "lnn_recovery_started",
      "description": "LNN recovery initiated",
      "event_count": 1,
      "mitigation": "LNN-FTC active",
    },
    {
      "id": "lnn_recovery_success",
      "description": "LNN recovery succeeded",
      "event_count": 1,
      "mitigation": "Motor loss survived",
    },
    {
      "id": "motor_loss_2",
      "description": "Motor 2 loss detected",
      "event_count": 1,
      "mitigation": "LNN-FTC recovery",
    },
    {
      "id": "pilot_override",
      "description": "Pilot override",
      "event_count": 1,
      "mitigation": "Pilot authority preserved",
    },
    {
      "id": "sensor_outlier",
      "description": "Sensor outlier detected",
      "event_count": 1,
      "mitigation": "SensorHealth warning",
    },
    {
      "id": "sensor_spoof",
      "description": "Sensor spoofing detected",
      "event_count": 1,
      "mitigation": "SpoofingDetector -> RTL",
    },
    {
      "id": "zombie_orbit",
      "description": "Zombie orbit detected",
      "event_count": 1,
      "mitigation": "ZombieDetector -> RTL latch",
    }
  ]
}
```
