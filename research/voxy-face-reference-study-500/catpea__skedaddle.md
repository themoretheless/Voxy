# Facial rendering source scan: catpea/skedaddle

Repository: https://github.com/catpea/skedaddle

Pinned source revision: c35d38d980a40e50ca265253070a59a4181b82cd

Discovery topic: tongue animation

Description: Hide secret messages in cat wink and tongue animations using International Morse Code

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: skedaddle.js

{
  "path": "skedaddle.js",
  "blob_sha": "9e7944383ac86babc97c6bda7d42341631dc45c6",
  "sha256": "198d6de01f53d28f6977dbf458076d5ccbf33c8c06a4eee9d154007a38bd7a60",
  "lines": 567,
  "bytes": 17361,
  "selection_score": 0,
  "keyword_line_counts": {
    "gum": 1,
    "tongue": 6,
    "face": 1
  },
  "url": "https://github.com/catpea/skedaddle/blob/c35d38d980a40e50ca265253070a59a4181b82cd/skedaddle.js",
  "function_signature_candidates": []
}

```text

#!/usr/bin/env node

import { spawn, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * SKEDADDLE
 * Encodes messages in International Morse Code using cat tongue animations
 *
 * In 100 years, a morse code expert will decode your cat video and discover:
 * "I HAS LICKS" or "FEED YOUR KITTY"
 *
 * Dit frame = Short signal (·)
 * Dah frame = Long signal (─)
 * Pause frame = Space between signals
 *
 * Inspired by: After Dark screensavers, Commodore Amiga intros, and cats
 */

class Skedaddle {
  constructor(options = {}) {
    const __dirname = path.dirname(fileURLToPath(import.meta.url));
    this.dit = options.dit || path.join(__dirname, 'samples', 'dit.jpg'); // Short signal (·)
    this.dah = options.dah || path.join(__dirname, 'samples', 'dah.jpg'); // Long signal (─)
    this.pause = options.pause || path.join(__dirname, 'samples', 'pause.jpg'); // Space/pause
    this.outputFile = options.output || 'morse.avif';
    this.text = options.text || 'MEOW';
    this.wpm = options.wpm || 15; // Words per minute (standard morse speed)
    this.format = options.format || 'avif'; // avif or mp4
    this.addAudio = options.addAudio || false;
    this.toneFrequency = options.toneFrequency || 800; // Hz
    this.endPause = options.endPause !== undefined ? options.endPause : 2.0; // Pause before loop (seconds)

    // Calculate timing based on WPM
    // Standard: PARIS = 50 units, WPM = (word count * 50) / (time in minutes * 60)
    // Therefore: 1 dit = 1200 / WPM milliseconds
    this.ditDuration = 1.2 / this.wpm; // in seconds
    this.dahDuration = this.ditDuration * 3;
    this.elementSpace = this.ditDuration; // space between dits/dahs in same letter
    this.letterSpace = this.ditDuration * 3; // space between letters
    this.wordSpace = this.ditDuration * 7; // space between words

    // International Morse Code
    this.morseTable = {
      'A': '.-',    'B': '-...',  'C': '-.-.',  'D': '-..',   'E': '.',
      'F': '..-.',  'G': '--.',   'H': '....',  'I': '..',    'J': '.---',
      'K': '-.-',   'L': '.-..',  'M': '--',    'N': '-.',    'O': '---',
      'P': '.--.',  'Q': '--.-',  'R': '.-.',   'S': '...',   'T': '-',
      'U': '..-',   'V': '...-',  'W': '.--',   'X': '-..-',  'Y': '-.--',
      'Z': '--..',
      '0': '-----', '1': '.----', '2': '..---', '3': '...--', '4': '....-',
      '5': '.....', '6': '-....', '7': '--...', '8': '---..', '9': '----.',
      '.': '.-.-.-', ',': '--..--', '?': '..--..', "'": '.----.',
      '!': '-.-.--', '/': '-..-.', '(': '-.--.', ')': '-.--.-',
      '&': '.-...', ':': '---...', ';': '-.-.-.', '=': '-...-',
      '+': '.-.-.', '-': '-....-', '_': '..--.-', '"': '.-..-.',
      '$': '...-..-', '@': '.--.-.',
      ' ': ' ' // Word space
    };
  }

  /**
   * Convert text to morse code
   */
  textToMorse(text) {
    return text.toUpperCase()
      .split('')
      .map(char => this.morseTable[char] || '')
      .filter(code => code !== '')
      .join(' ');
  }

  /**
   * Generate frame sequence from morse code
   */
  generateSequence() {
    const morse = this.textToMorse(this.text);
    const sequence = [];

    console.log(`📻 Morse Code: ${morse}\n`);

    let position = 0;
    const morseChars = morse.split('');

    for (let i = 0; i < morseChars.length; i++) {
      const char = morseChars[i];

      if (char === '.') {
        // Dit - short signal (tongue out)
        sequence.push({
          frame: this.dit,
          duration: this.ditDuration,
          type: 'dit',
          symbol: '·'
        });
        position += this.ditDuration;

      } else if (char === '-') {
        // Dah - long signal
        sequence.push({
          frame: this.dah,
          duration: this.dahDuration,
          type: 'dah',
          symbol: '─'
        });
        position += this.dahDuration;

      } else if (char === ' ') {
        // Check if next char is also space (word boundary)
        if (morseChars[i + 1] === ' ') {
          // Word space (already have letter space from previous)
          sequence.push({
            frame: this.pause,
            duration: this.wordSpace - this.letterSpace,
            type: 'word-space',
            symbol: '   '
          });
          position += this.wordSpace - this.letterSpace;
          i++; // Skip next space
        } else {
          // Letter space
          sequence.push({
            frame: this.pause,
            duration: this.letterSpace,
            type: 'letter-space',
            symbol: ' '
          });
          position += this.letterSpace;
        }
        continue;
      }

      // Add element space (between dits/dahs in same letter)
      // But not after last element
      if (i < morseChars.length - 1 && morseChars[i + 1] !== ' ') {
        sequence.push({
          frame: this.pause,
          duration: this.elementSpace,
          type: 'element-space',
          symbol: ''
        });
        position += this.elementSpace;
      }
    }

    // Add end pause before loop (helps learners distinguish message boundaries)
    if (this.endPause > 0) {
      sequence.push({
        frame: this.pause,
        duration: this.endPause,
        type: 'end-pause',
        symbol: ''
      });
      position += this.endPause;
    }

    return { sequence, totalDuration: position };
  }

  /**
   * Create ffmpeg concat file
   */
  createConcatFile(sequence) {
    const concatPath = '/tmp/morse-concat.txt';
    let content = '';

    for (const item of sequence) {
      content += `file '${path.resolve(item.frame)}'\n`;
      content += `duration ${item.duration.toFixed(4)}\n`;
    }

    // Add last frame again (ffmpeg concat quirk)
    const lastFrame = sequence[sequence.length - 1];
    content += `file '${path.resolve(lastFrame.frame)}'\n`;

    fs.writeFileSync(concatPath, content);
    return concatPath;
  }

  /**
   * Generate audio beep track
   */
  async generateAudio(sequence, totalDuration) {
    console.log('🔊 Generating morse audio...\n');

    const audioFile = '/tmp/morse-audio.wav';
    let filterComplex = `anullsrc=r=44100:cl=mono[silent];`;
    let currentTime = 0;
    const beeps = [];

    for (const item of sequence) {
      if (item.type === 'dit' || item.type === 'dah') {
        beeps.push(
          `sine=f=${this.toneFrequency}:d=${item.duration}[beep${beeps.length}];` +
          `[beep${beeps.length}]adelay=${Math.round(currentTime * 1000)}|${Math.round(currentTime * 1000)}[delayed${beeps.length}]`
        );
      }
      currentTime += item.duration;
    }

    filterComplex += beeps.join(';') + ';';
    filterComplex += `[silent]${beeps.map((_, i) => `[delayed${i}]`).join('')}amix=inputs=${beeps.length + 1}:duration=longest[out]`;

    const args = [
      '-f', 'lavfi',
      '-i', 'anullsrc=r=44100:cl=mono',
      '-filter_complex', filterComplex,
      '-map', '[out]',
      '-t', totalDuration.toFixed(4),
      '-y',
      audioFile
    ];

    return new Promise((resolve, reject) => {
      const ffmpeg = spawn('ffmpeg', args);

      let stderr = '';
      ffmpeg.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      ffmpeg.on('close', (code) => {
        if (code === 0) {
          resolve(audioFile);
        } else {
          console.error('Audio generation failed:', stderr);
          reject(new Error('Audio generation failed'));
        }
      });
    });
  }

  /**
   * Create the morse animation
   */
  async create() {
    console.log('📡 SKEDADDLE ENCODER');
    console.log('='.repeat(60));
    console.log(`   Secret Message: "${this.text}"`);
    console.log(`   Speed: ${this.wpm} WPM`);
    console.log('='.repeat(60) + '\n');

    // Check frames exist
    if (!fs.existsSync(this.dit)) {
      throw new Error(`Dit frame not found: ${this.dit}`);
    }
    if (!fs.existsSync(this.dah)) {
      throw new Error(`Dah frame not found: ${this.dah}`);
    }
    if (!fs.existsSync(this.pause)) {
      throw new Error(`Pause frame not found: ${this.pause}`);
    }

    // Generate sequence
    const { sequence, totalDuration } = this.generateSequence();

    console.log('📊 Transmission Plan:');
    console.log('─'.repeat(60));

    // Display in a readable format
    let visualMorse = '';
    let currentLetter = [];

    sequence.forEach((item, i) => {
      if (item.type === 'dit' || item.type === 'dah') {
        visualMorse += item.symbol;
        currentLetter.push(item.symbol);
      } else if (item.type === 'letter-space') {
        visualMorse += ' ';
        currentLetter = [];
      } else if (item.type === 'word-space') {
        visualMorse += '   ';
        currentLetter = [];
      }
    });

    console.log(`   ${visualMorse}`);
    console.log('─'.repeat(60));
    console.log(`   Duration: ${totalDuration.toFixed(2)}s`);
    console.log(`   Frames: ${sequence.length}`);
    console.log(`   Dit: ${(this.ditDuration * 1000).toFixed(0)}ms`);
    console.log(`   Dah: ${(this.dahDuration * 1000).toFixed(0)}ms\n`);

    // Create concat file
    const concatFile = this.createConcatFile(sequence);

    // Generate audio if requested
    let audioFile = null;
    if (this.addAudio) {
      audioFile = await this.generateAudio(sequence, totalDuration);
    }

    // Build ffmpeg command
    const args = [
      '-f', 'concat',
      '-safe', '0',
      '-i', concatFile
    ];

    if (audioFile) {
      args.push('-i', audioFile);
    }

    if (this.format === 'avif') {
      args.push(
        '-vf', 'format=yuv420p',
        '-c:v', 'libaom-av1',
        '-still-picture', '0',
        '-loop', '0',
        '-cpu-used', '8'
      );
      if (audioFile) {
        args.push('-c:a', 'libopus');
      }
    } else {
      args.push(
        '-vf', 'format=yuv420p',
        '-c:v', 'libx264',
        '-preset', 'medium',
        '-crf', '23'
      );
      if (audioFile) {
        args.push('-c:a', 'aac');
      }
    }

    args.push('-y', this.outputFile);

    console.log('🎬 Encoding transmission...\n');

    return new Promise((resolve, reject) => {
      const ffmpeg = spawn('ffmpeg', args);

      let stderr = '';

      ffmpeg.stderr.on('data', (data) => {
        stderr += data.toString();
        if (data.toString().includes('frame=')) {
          process.stdout.write('.');
        }
      });

      ffmpeg.on('close', (code) => {
        console.log('\n');
        if (code === 0) {
          // Cleanup
          fs.unlinkSync(concatFile);
          if (audioFile) {
            try { fs.unlinkSync(audioFile); } catch (e) {}
          }

          const stats = fs.statSync(this.outputFile);
          const sizeKB = (stats.size / 1024).toFixed(2);

          console.log('✅ Transmission encoded!');
          console.log(`📦 Output: ${this.outputFile} (${sizeKB} KB)`);
          console.log(`⏱️  Duration: ${totalDuration.toFixed(2)}s`);
          console.log(`📻 Message: "${this.text}"`);
          console.log(`🔐 Morse: ${this.textToMorse(this.text)}`);

          resolve();
        } else {
          console.error('❌ Encoding failed:');
          console.error(stderr);
          reject(new Error('ffmpeg encoding failed'));
        }
      });

      ffmpeg.on('error', reject);
    });
  }

  /**
   * Decode morse from video (theoretical - for documentation)
   */
  static decodeInstructions() {
    return `
📡 DECODING SKEDADDLE

To decode a Morse Kitty transmission:

1. Watch the video frame by frame
2. Note when tongue appears (signal)
3. Measure duration:
   - Short tongue = Dit (·)
   - Long tongue = Dah (─)
4. Note pauses:
   - Short pause = element space
   - Medium pause = letter boundary
   - Long pause = word boundary
5. Translate morse to text using standard table

Or just ask the grandpa who decoded Enigma transmissions in 1943.
He'll figure it out immediately and have flashbacks.

Standard WPM speeds:
- 5 WPM: Beginner
- 15 WPM: Standard (default)
- 25 WPM: Proficient
- 40+ WPM: Expert military operators

At 15 WPM, each dit is approximately 80ms.
    `;
  }
}


function checkFfmpeg() {
  try {
    execSync('ffmpeg -version', { stdio: 'ignore' });
    return true;
  } catch (err) {
    return false;
  }
}

// CLI interface - handle both direct execution and npm bin symlinks
const __filename = fileURLToPath(import.meta.url);
const isMainModule = process.argv[1] && fs.realpathSync(process.argv[1]) === fs.realpathSync(__filename);

if (isMainModule) {

  // Check for ffmpeg
  if (!checkFfmpeg()) {
    console.error('❌ Error: ffmpeg not found!');
    console.error('\nSkedaddle requires ffmpeg to be installed.');
    console.error('\nTo install ffmpeg:');
    console.error('  • Ubuntu/Debian: sudo apt install ffmpeg');
    console.error('  • Fedora: sudo dnf install ffmpeg');
    console.error('  • Arch: sudo pacman -S ffmpeg');
    console.error('  • macOS: brew install ffmpeg');
    console.error('\nVisit https://ffmpeg.org for more information.\n');
    process.exit(1);
  }


  const args = process.argv.slice(2);

  const options = {
    output: 'morse.avif',
    text: 'MEOW',
    wpm: 15,
    format: 'avif',
    addAudio: false,
    toneFrequency: 800
  };

  // Parse arguments
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--text' || args[i] === '-t') {
      options.text = args[++i];
    } else if (args[i] === '--wpm' || args[i] === '-w') {
      options.wpm = parseInt(args[++i]);
    } else if (args[i] === '--output' || args[i] === '-o') {
      options.output = args[++i];
    } else if (args[i] === '--format' || args[i] === '-f') {
      options.format = args[++i];
    } else if (args[i] === '--audio' || args[i] === '-a') {
      options.addAudio = true;
    } else if (args[i] === '--frequency' || args[i] === '-F') {
      options.toneFrequency = parseInt(args[++i]);
    } else if (args[i] === '--dit' || args[i] === '-D') {
      options.dit = args[++i];
    } else if (args[i] === '--dah' || args[i] === '-H') {
      options.dah = args[++i];
    } else if (args[i] === '--pause' || args[i] === '-P') {
      options.pause = args[++i];
    } else if (args[i] === '--end-pause' || args[i] === '-E') {
      options.endPause = parseFloat(args[++i]);
    } else if (args[i] === '--decode-help') {
      console.log(Skedaddle.decodeInstructions());
      process.exit(0);
    } else if (args[i] === '--help' || args[i] === '-h') {
      console.log(`
SKEDADDLE - Secret messages in cat licks
==========================================

Encodes text into International Morse Code using cat tongue animations.
Perfect for hiding messages in plain sight, confusing grandpas with PTSD,
and paying homage to the Commodore Amiga demo scene.

Usage: skedaddle [options]

OPTIONS:
  -t, --text <message>       Message to encode (default: "MEOW")
  -w, --wpm <speed>          Words per minute (default: 15)
  -o, --output <file>        Output filename (default: morse.avif)
  -f, --format <fmt>         Output format: avif or mp4 (default: avif)
  -a, --audio                Add morse audio beep track
  -F, --frequency <hz>       Beep frequency in Hz (default: 800)
  -D, --dit <file>           Dit image - short signal (default: ./samples/dit.jpg)
  -H, --dah <file>           Dah image - long signal (default: ./samples/dah.jpg)
  -P, --pause <file>         Pause image - space/pause (default: ./samples/pause.jpg)

  --decode-help              Show instructions for decoding
  -h, --help                 Show this help

EXAMPLES:

  # Basic usage
  skedaddle --text "I HAS LICKS"

  # Faster transmission
  skedaddle --text "FEED YOUR KITTY" --wpm 25

  # With audio beeps (so grandpa can hear it)
  skedaddle --text "SOS" --audio --format mp4

  # Custom frames and frequency
  skedaddle -t "MEOW MEOW" -D dit.jpg -H dah.jpg -P pause.jpg -F 1000

  # Professional speed for experts
  skedaddle -t "THE QUICK BROWN FOX" -w 40 -o secret.mp4

TIMING:
  At 15 WPM (standard):
  - Dit (·): ~80ms
  - Dah (─): ~240ms
  - Element space: ~80ms
  - Letter space: ~240ms
  - Word space: ~560ms

SUPPORTED CHARACTERS:
  Letters: A-Z
  Numbers: 0-9
  Punctuation: . , ? ' ! / ( ) & : ; = + - _ " $ @

HISTORICAL NOTE:
  International Morse Code has been used since 1848.
  Your cat is now part of this proud tradition.

  In 100 years, some historian will decode your cat video
  and discover the message "I HAS LICKS" - contributing
  significantly to our understanding of 21st century culture.

TRIVIA:
  - SOS is "... --- ..." (easy to remember)
  - "PARIS" is the standard 50-unit word for WPM calculation
  - The letter E (·) is the shortest morse code
  - Numbers are longest (5 elements each)

FOR THE DEMO SCENE:
  This is in the spirit of Commodore Amiga intros,
  After Dark screensavers, and hidden messages in art.
  We don't need AI. We have cats and morse code.
      `);
      process.exit(0);
    }
  }

  const morse = new Skedaddle(options);

  morse.create()
    .then(() => {
      console.log('\n🎉 Transmission complete!');
      console.log('   Your secret message is safely hidden in cat licks.');
      console.log('   Only morse code experts will ever know... 📡\n');
    })
    .catch((err) => {
      console.error('\n💥 Error:', err.message);
      process.exit(1);
    });
}

export default Skedaddle;


```
