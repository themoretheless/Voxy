# Facial rendering source scan: alexandergmzx/mouth_animator

Repository: https://github.com/alexandergmzx/mouth_animator

Pinned source revision: e4197cce4fba702ddbec357c27d92397a949718c

Discovery topic: mouth animation

Description: I am making a mouth animation for an animatronic

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"mouth": ["Billy_Mouth.ino", "MouthAnimator.h"]}

## Source file: MouthAnimator.h

{
  "path": "MouthAnimator.h",
  "blob_sha": "99af9c90125e1c0e0e78db6d3f7d2c6ac4f1cda5",
  "sha256": "ebeab41961f3f3edf7f9d88dfde4652405de5b632e61111602d000601fbfc083",
  "lines": 352,
  "bytes": 9044,
  "selection_score": 15,
  "keyword_line_counts": {
    "mouth": 6
  },
  "url": "https://github.com/alexandergmzx/mouth_animator/blob/e4197cce4fba702ddbec357c27d92397a949718c/MouthAnimator.h",
  "function_signature_candidates": [
    {
      "line": 56,
      "text": "void setSpeed(float multiplier) {"
    },
    {
      "line": 63,
      "text": "void setText(String text) {"
    },
    {
      "line": 123,
      "text": "void play() {"
    },
    {
      "line": 139,
      "text": "void stop() {"
    },
    {
      "line": 146,
      "text": "void update() {"
    },
    {
      "line": 168,
      "text": "bool isAnimating() const { return animating; }"
    },
    {
      "line": 170,
      "text": "void printSequence() {"
    },
    {
      "line": 184,
      "text": "void parseCharacter(char c) {"
    },
    {
      "line": 289,
      "text": "void addMovement(int position, unsigned long duration) {"
    },
    {
      "line": 298,
      "text": "void addSustained(int position, unsigned long duration) {"
    },
    {
      "line": 303,
      "text": "void addExplosive(int maxPosition, unsigned long totalDuration) {"
    },
    {
      "line": 316,
      "text": "void addFlutter(int position, unsigned long totalDuration) {"
    },
    {
      "line": 327,
      "text": "void executeCurrentStep() {"
    }
  ]
}

```text

// =============================================================================
// FILE: MouthAnimator.h
// Spanish phoneme-based mouth animation engine
// =============================================================================
#ifndef MOUTH_ANIMATOR_H
#define MOUTH_ANIMATOR_H

#include <Arduino.h>
#include "Config.h"
#include "MotorControl.h"

// Movement step structure
struct MovementStep {
  int targetPosition;  // 0-3
  unsigned long duration;  // milliseconds
};

// Position definitions with PWM and timing
struct PositionConfig {
  int pwm;
  unsigned long time_ms;
};

class MouthAnimator {
private:
  MotorControl* motor;
  
  // Position configurations (from your calibration)
  PositionConfig positions[4] = {
    {0, 0},        // Position 0: Closed
    {255, 20},     // Position 1: Slight open
    {255, 80},     // Position 2: Medium open
    {255, 200}     // Position 3: Wide open
  };
  
  // Movement sequence buffer
  static const int MAX_SEQUENCE_LENGTH = 200;
  MovementStep sequence[MAX_SEQUENCE_LENGTH];
  int sequenceLength;
  int currentStep;
  
  // Animation state
  bool animating;
  unsigned long stepStartTime;
  int currentPosition;
  
  // Speed multiplier for exaggeration
  float speedMultiplier;
  
public:
  MouthAnimator(MotorControl* motorController) 
    : motor(motorController), sequenceLength(0), currentStep(0),
      animating(false), currentPosition(0), speedMultiplier(1.0) {}
  
  // Set speed (1.0 = normal, 1.5 = faster, 0.7 = slower)
  void setSpeed(float multiplier) {
    speedMultiplier = multiplier;
    Serial.print("Animation speed set to: ");
    Serial.println(speedMultiplier);
  }
  
  // Convert Spanish text to movement sequence
  void setText(String text) {
    text.toUpperCase();
    text.trim();
    
    if (text.length() == 0) {
      Serial.println("ERROR: Empty text");
      return;
    }
    
    Serial.print("Parsing text: \"");
    Serial.print(text);
    Serial.println("\"");
    
    sequenceLength = 0;
    currentStep = 0;
    currentPosition = 0;
    
    // Parse text into phoneme sequence
    for (unsigned int i = 0; i < text.length() && sequenceLength < MAX_SEQUENCE_LENGTH - 1; i++) {
      char c = text.charAt(i);
      
      // Handle special two-character phonemes
      if (i < text.length() - 1) {
        char next = text.charAt(i + 1);
        
        // CH
        if (c == 'C' && next == 'H') {
          addExplosive(2, 100);  // CH: medium snap
          i++; // Skip next char
          continue;
        }
        // LL
        if (c == 'L' && next == 'L') {
          addSustained(2, 80);  // LL: medium sustained
          i++;
          continue;
        }
        // RR (trilled R)
        if (c == 'R' && next == 'R') {
          addFlutter(2, 120);  // RR: flutter effect
          i++;
          continue;
        }
      }
      
      // Single character phonemes
      parseCharacter(c);
    }
    
    // Always end closed
    addMovement(0, 100);
    
    Serial.print("Generated ");
    Serial.print(sequenceLength);
    Serial.println(" movement steps");
    
    printSequence();
  }
  
  // Start animation playback
  void play() {
    if (sequenceLength == 0) {
      Serial.println("ERROR: No animation loaded. Use SAY command first.");
      return;
    }
    
    animating = true;
    currentStep = 0;
    currentPosition = 0;
    stepStartTime = millis();
    
    Serial.println("Animation started!");
    executeCurrentStep();
  }
  
  // Stop animation
  void stop() {
    animating = false;
    motor->stop();
    Serial.println("Animation stopped");
  }
  
  // Update animation (call in loop)
  void update() {
    if (!animating) return;
    
    unsigned long elapsed = millis() - stepStartTime;
    unsigned long stepDuration = sequence[currentStep].duration / speedMultiplier;
    
    if (elapsed >= stepDuration) {
      currentStep++;
      
      if (currentStep >= sequenceLength) {
        // Animation complete
        animating = false;
        motor->stop();
        Serial.println("Animation complete!");
        return;
      }
      
      stepStartTime = millis();
      executeCurrentStep();
    }
  }
  
  bool isAnimating() const { return animating; }
  
  void printSequence() {
    Serial.println("\n=== Movement Sequence ===");
    for (int i = 0; i < sequenceLength; i++) {
      Serial.print(i);
      Serial.print(": Pos ");
      Serial.print(sequence[i].targetPosition);
      Serial.print(" for ");
      Serial.print(sequence[i].duration);
      Serial.println(" ms");
    }
    Serial.println("=========================\n");
  }
  
private:
  void parseCharacter(char c) {
    switch (c) {
      // VOWELS
      case 'A':
        addSustained(3, 200);  // Wide open
        break;
      case 'E':
        addSustained(2, 150);  // Medium
        break;
      case 'I':
        addSustained(1, 120);  // Narrow
        break;
      case 'O':
        addSustained(3, 180);  // Wide round
        break;
      case 'U':
        addSustained(1, 120);  // Small round
        break;
      
      // BILABIAL CONSONANTS (explosive)
      case 'P':
      case 'B':
      case 'M':
        addExplosive(3, 80);  // Big snap
        break;
      
      // DENTAL/ALVEOLAR (quick tap)
      case 'T':
      case 'D':
        addExplosive(2, 60);  // Quick tap
        break;
      
      // FRICATIVES (sustained narrow)
      case 'S':
      case 'Z':
      case 'F':
        addSustained(1, 80);  // Hiss
        break;
      
      // NASALS/LIQUIDS (sustained medium)
      case 'N':
      case 'L':
      case 'R':  // Single R
        addSustained(2, 100);
        break;
      
      // VELAR (sharp closure)
      case 'K':
      case 'Q':
      case 'G':
        addExplosive(2, 70);
        break;
      
      // SPECIAL
      case 'J':  // Jota (fricative)
      case 'X':
        addSustained(1, 90);
        break;
      
      case 'C':  // Context-dependent, default to K sound
        addExplosive(2, 70);
        break;
      
      case 'V':  // Same as B in Spanish
        addExplosive(3, 80);
        break;
      
      case 'Y':
        addSustained(2, 80);
        break;
      
      // SPECIAL CHARACTERS
      case ' ':
      case ',':
      case '.':
      case '!':
      case '?':
        addMovement(0, 150);  // Pause
        break;
      
      // Accented vowels
      case 'Á':
        addSustained(3, 220);  // Emphasized A
        break;
      case 'É':
        addSustained(2, 170);
        break;
      case 'Í':
        addSustained(1, 140);
        break;
      case 'Ó':
        addSustained(3, 200);
        break;
      case 'Ú':
      case 'Ü':
        addSustained(1, 140);
        break;
      
      default:
        // Ignore unknown characters
        break;
    }
  }
  
  // Add simple movement
  void addMovement(int position, unsigned long duration) {
    if (sequenceLength >= MAX_SEQUENCE_LENGTH) return;
    
    sequence[sequenceLength].targetPosition = position;
    sequence[sequenceLength].duration = duration;
    sequenceLength++;
  }
  
  // Add sustained position (vowels, fricatives)
  void addSustained(int position, unsigned long duration) {
    addMovement(position, duration);
  }
  
  // Add explosive movement (consonants: closed -> open -> closed)
  void addExplosive(int maxPosition, unsigned long totalDuration) {
    unsigned long closeDuration = totalDuration * 0.2;  // 20% closing
    unsigned long openDuration = totalDuration * 0.3;   // 30% opening
    unsigned long holdDuration = totalDuration * 0.2;   // 20% hold
    unsigned long returnDuration = totalDuration * 0.3; // 30% return
    
    addMovement(0, closeDuration);           // Close
    addMovement(maxPosition, openDuration);  // Snap open
    addMovement(maxPosition, holdDuration);  // Brief hold
    addMovement(0, returnDuration);          // Return closed
  }
  
  // Add flutter effect (for RR)
  void addFlutter(int position, unsigned long totalDuration) {
    // Rapid open/close cycles
    int cycles = 3;
    unsigned long cycleDuration = totalDuration / cycles;
    
    for (int i = 0; i < cycles; i++) {
      addMovement(position, cycleDuration / 2);
      addMovement(0, cycleDuration / 2);
    }
  }
  
  void executeCurrentStep() {
    int targetPos = sequence[currentStep].targetPosition;
    
    // If moving to a different position
    if (targetPos != currentPosition) {
      if (targetPos > currentPosition) {
        // Opening
        motor->forward(positions[targetPos].pwm, positions[targetPos].time_ms);
      } else {
        // Closing - can rely on spring or use reverse
        if (targetPos == 0) {
          // Let spring close naturally (faster with small assist)
          motor->reverse(100, 50);  // Brief reverse assist
        } else {
          // Partial close to intermediate position
          motor->reverse(200, positions[targetPos].time_ms);
        }
      }
      currentPosition = targetPos;
    }
    // If staying at same position, motor stays off (holding)
  }
};

#endif



```

## Source file: Config.h

{
  "path": "Config.h",
  "blob_sha": "a2a24a89181e8480fee748ed35c2e1b2e6a36a63",
  "sha256": "e72e0801d287252c7bd95fcbf975f2ebd8503ff1866b8aeee70c49113ebcb291",
  "lines": 35,
  "bytes": 1364,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alexandergmzx/mouth_animator/blob/e4197cce4fba702ddbec357c27d92397a949718c/Config.h",
  "function_signature_candidates": []
}

```text

// =============================================================================
// FILE: Config.h
// All configuration constants and global settings
// =============================================================================
#ifndef CONFIG_H
#define CONFIG_H

// ===== PIN CONFIGURATION =====
const int MOTOR_IN1 = 5;        // L298N IN1
const int MOTOR_IN2 = 6;        // L298N IN2
const int MOTOR_ENA = 9;        // L298N ENA (PWM)
const int CURRENT_SENSOR_PIN = A0;  // ACS712 output

// ===== CURRENT SENSOR CALIBRATION (ACS712) =====
const float ACS712_ZERO_VOLTAGE = 2.5;  // Voltage at 0A
const float ACS712_SENSITIVITY = 0.1;   // 100mV per Amp (20A version)
const float ADC_VREF = 5.0;             // Arduino reference voltage
const int ADC_RESOLUTION = 16384;       // 14-bit ADC on R4 WiFi (2^14)
const int CURRENT_SAMPLES = 5;          // Samples for moving average

// ===== SAFETY LIMITS =====
const float DEFAULT_CURRENT_LIMIT = 3.0;    // Default max current (Amps)
const unsigned long MAX_RUN_TIME = 5000;    // Max continuous run time (ms)
const int MAX_PWM = 255;                    // Maximum PWM value

// ===== DATA LOGGING =====
const unsigned long DEFAULT_LOG_INTERVAL = 50;  // Default log interval (ms)

// ===== MOTOR DIRECTIONS =====
const char DIR_FORWARD = 'F';
const char DIR_REVERSE = 'R';
const char DIR_STOP = 'S';

#endif



```

## Source file: DataLogger.h

{
  "path": "DataLogger.h",
  "blob_sha": "26a1fac5de8fdb89957cceebfa795c37775b1426",
  "sha256": "56da104e067e8653e200e0444c0318b4ad9696b95f1d867d69564386388c2850",
  "lines": 75,
  "bytes": 1820,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alexandergmzx/mouth_animator/blob/e4197cce4fba702ddbec357c27d92397a949718c/DataLogger.h",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "void enable() {"
    },
    {
      "line": 27,
      "text": "void disable() {"
    },
    {
      "line": 32,
      "text": "void setInterval(unsigned long ms) {"
    },
    {
      "line": 43,
      "text": "void update(float current, MotorControl& motor) {"
    },
    {
      "line": 51,
      "text": "bool isEnabled() const { return enabled; }"
    },
    {
      "line": 55,
      "text": "void printHeader() {"
    },
    {
      "line": 60,
      "text": "void logData(float current, MotorControl& motor) {"
    }
  ]
}

```text

// =============================================================================
// FILE: DataLogger.h
// CSV data logging functionality
// =============================================================================
#ifndef DATA_LOGGER_H
#define DATA_LOGGER_H

#include <Arduino.h>
#include "Config.h"
#include "MotorControl.h"

class DataLogger {
private:
  bool enabled;
  unsigned long interval;
  unsigned long lastLogTime;
  
public:
  DataLogger() : enabled(false), interval(DEFAULT_LOG_INTERVAL), lastLogTime(0) {}
  
  void enable() {
    enabled = true;
    printHeader();
    Serial.println("Data logging ENABLED");
  }
  
  void disable() {
    enabled = false;
    Serial.println("Data logging DISABLED");
  }
  
  void setInterval(unsigned long ms) {
    if (ms >= 10 && ms <= 1000) {
      interval = ms;
      Serial.print("Log interval set to: ");
      Serial.print(interval);
      Serial.println(" ms");
    } else {
      Serial.println("ERROR: Interval must be 10-1000 ms");
    }
  }
  
  void update(float current, MotorControl& motor) {
    if (!enabled) return;
    if (millis() - lastLogTime < interval) return;
    
    logData(current, motor);
    lastLogTime = millis();
  }
  
  bool isEnabled() const { return enabled; }
  unsigned long getInterval() const { return interval; }
  
private:
  void printHeader() {
    Serial.println("\n--- DATA LOG START ---");
    Serial.println("timestamp_ms,pwm,current_A,direction,motor_running");
  }
  
  void logData(float current, MotorControl& motor) {
    Serial.print(millis());
    Serial.print(',');
    Serial.print(motor.getPWM());
    Serial.print(',');
    Serial.print(current, 4);
    Serial.print(',');
    Serial.print(motor.getDirection());
    Serial.print(',');
    Serial.println(motor.isRunning() ? '1' : '0');
  }
};

#endif




```

## Source file: MotorControl.h

{
  "path": "MotorControl.h",
  "blob_sha": "675fd356e5a04ed07b0d6068c0c70a87635c2564",
  "sha256": "bdd6e06f69177d605ea728a34b84e109f9fd464f2ed2b1017888a944a4fac92d",
  "lines": 164,
  "bytes": 4214,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/alexandergmzx/mouth_animator/blob/e4197cce4fba702ddbec357c27d92397a949718c/MotorControl.h",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "void begin() {"
    },
    {
      "line": 35,
      "text": "void forward(int pwm, unsigned long duration = 0) {"
    },
    {
      "line": 57,
      "text": "void reverse(int pwm, unsigned long duration = 0) {"
    },
    {
      "line": 79,
      "text": "void stop() {"
    },
    {
      "line": 88,
      "text": "void update(float current) {"
    },
    {
      "line": 122,
      "text": "void setCurrentLimit(float limit) {"
    },
    {
      "line": 134,
      "text": "bool isRunning() const { return running; }"
    },
    {
      "line": 135,
      "text": "int getPWM() const { return currentPWM; }"
    },
    {
      "line": 138,
      "text": "float getCurrentLimit() const { return currentLimit; }"
    },
    {
      "line": 140,
      "text": "void printStatus() {"
    }
  ]
}

```text

// =============================================================================
// FILE: MotorControl.h
// Motor control and safety management
// =============================================================================
#ifndef MOTOR_CONTROL_H
#define MOTOR_CONTROL_H

#include <Arduino.h>
#include "Config.h"

class MotorControl {
private:
  int pinIN1, pinIN2, pinENA;
  bool running;
  unsigned long startTime;
  int currentPWM;
  char direction;
  unsigned long targetRunTime;
  float currentLimit;
  
public:
  MotorControl(int in1, int in2, int ena) 
    : pinIN1(in1), pinIN2(in2), pinENA(ena),
      running(false), startTime(0), currentPWM(0), 
      direction(DIR_STOP), targetRunTime(0), 
      currentLimit(DEFAULT_CURRENT_LIMIT) {}
  
  void begin() {
    pinMode(pinIN1, OUTPUT);
    pinMode(pinIN2, OUTPUT);
    pinMode(pinENA, OUTPUT);
    stop();
  }
  
  void forward(int pwm, unsigned long duration = 0) {
    currentPWM = pwm;
    direction = DIR_FORWARD;
    running = true;
    startTime = millis();
    targetRunTime = duration;
    
    digitalWrite(pinIN1, HIGH);
    digitalWrite(pinIN2, LOW);
    analogWrite(pinENA, pwm);
    
    Serial.print("Motor FORWARD at PWM ");
    Serial.print(pwm);
    if (duration > 0) {
      Serial.print(" for ");
      Serial.print(duration);
      Serial.println(" ms");
    } else {
      Serial.println(" (manual stop required)");
    }
  }
  
  void reverse(int pwm, unsigned long duration = 0) {
    currentPWM = pwm;
    direction = DIR_REVERSE;
    running = true;
    startTime = millis();
    targetRunTime = duration;
    
    digitalWrite(pinIN1, LOW);
    digitalWrite(pinIN2, HIGH);
    analogWrite(pinENA, pwm);
    
    Serial.print("Motor REVERSE at PWM ");
    Serial.print(pwm);
    if (duration > 0) {
      Serial.print(" for ");
      Serial.print(duration);
      Serial.println(" ms");
    } else {
      Serial.println(" (manual stop required)");
    }
  }
  
  void stop() {
    digitalWrite(pinIN1, LOW);
    digitalWrite(pinIN2, LOW);
    analogWrite(pinENA, 0);
    running = false;
    direction = DIR_STOP;
    targetRunTime = 0;
  }
  
  void update(float current) {
    if (!running) return;
    
    unsigned long runtime = millis() - startTime;
    
    // Check if timed run is complete
    if (targetRunTime > 0 && runtime >= targetRunTime) {
      stop();
      Serial.println("\nTimed run complete - motor stopped");
      Serial.print("Final current: ");
      Serial.print(current, 3);
      Serial.println(" A");
      return;
    }
    
    // Safety: Current limit
    if (current > currentLimit) {
      stop();
      Serial.println("\n!!! STOPPED: Current limit exceeded !!!");
      Serial.print("Peak current: ");
      Serial.print(current, 3);
      Serial.print(" A (limit: ");
      Serial.print(currentLimit);
      Serial.println(" A)");
      return;
    }
    
    // Safety: Maximum runtime
    if (runtime > MAX_RUN_TIME) {
      stop();
      Serial.println("\n!!! STOPPED: Maximum run time (5s) !!!");
    }
  }
  
  void setCurrentLimit(float limit) {
    if (limit > 0 && limit <= 5.0) {
      currentLimit = limit;
      Serial.print("Current limit set to: ");
      Serial.print(currentLimit);
      Serial.println(" A");
    } else {
      Serial.println("ERROR: Limit must be 0.1 - 5.0 A");
    }
  }
  
  // Getters
  bool isRunning() const { return running; }
  int getPWM() const { return currentPWM; }
  char getDirection() const { return direction; }
  unsigned long getRuntime() const { return running ? millis() - startTime : 0; }
  float getCurrentLimit() const { return currentLimit; }
  
  void printStatus() {
    Serial.println("\n===== MOTOR STATUS =====");
    Serial.print("State: ");
    Serial.println(running ? "RUNNING" : "STOPPED");
    
    if (running) {
      Serial.print("  Direction: ");
      Serial.println(direction);
      Serial.print("  PWM: ");
      Serial.println(currentPWM);
      Serial.print("  Runtime: ");
      Serial.print(getRuntime());
      Serial.println(" ms");
    }
    
    Serial.print("Current limit: ");
    Serial.print(currentLimit);
    Serial.println(" A");
    Serial.println("========================\n");
  }
};

#endif




```
