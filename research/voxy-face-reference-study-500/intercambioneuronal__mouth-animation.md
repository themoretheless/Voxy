# Facial rendering source scan: intercambioneuronal/mouth-animation

Repository: https://github.com/intercambioneuronal/mouth-animation

Pinned source revision: f4e21e503cb6ae1ae6ffa79d93262e82624234bf

Discovery topic: mouth animation

Description: A simple mouth animation using canvas.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"mouth": ["animation-mouth-english.png", "preview-mouth-animation.png"]}

## Source file: src/Util.ts

{
  "path": "src/Util.ts",
  "blob_sha": "e51c2596d435ac3ddd84e4fc1c61c449b971232b",
  "sha256": "9052762db706ea34df648806bddbe803e678a7d8923dcb9de7533a40b7956307",
  "lines": 7,
  "bytes": 139,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/intercambioneuronal/mouth-animation/blob/f4e21e503cb6ae1ae6ffa79d93262e82624234bf/src/Util.ts",
  "function_signature_candidates": []
}

```text

export class Util{
    static delay(callback:Function,ms:number){
        setTimeout(() => {
            callback()
        }, ms);
    }
}

```

## Source file: src/index.ts

{
  "path": "src/index.ts",
  "blob_sha": "ef4bdadbc6dbf2f170b2e175911dff3573165c5d",
  "sha256": "3b2206b09617f34e7bba567840db509fb3a6136bc76a190f577e32b28cdcc23e",
  "lines": 60,
  "bytes": 2347,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/intercambioneuronal/mouth-animation/blob/f4e21e503cb6ae1ae6ffa79d93262e82624234bf/src/index.ts",
  "function_signature_candidates": []
}

```text

import { CanvasManager } from "./CanvasManager.js";
import { ContextGenerator2D } from "./ContextGenerator2D.js";
import { DrawerImage2D } from "./DrawerImage2D.js";
import { ICanvas } from "./iCanvas.js";
import sprite from '../sprites.json' assert{"type": "json"};
import { InfoImage } from "./types.js";
import { SpriteManager } from "./SpriteManager.js";
import { ClearerImage2D } from "./ClearerImage2D.js";
import { Util } from "./Util.js"
import { Speaker } from "./Speaker.js";

let datosCanvas: ICanvas = {
    idCanvas: "dibujando",
    type: "2d"
}
let spriteAfor: InfoImage[] = sprite as InfoImage[]
let img: HTMLElement | null = document.getElementById('afor')
let elementoTexto: HTMLElement | null = document.getElementById('texto')
let botonHablar: HTMLElement | null = document.getElementById('hablar')

let contextGenerator2D = new ContextGenerator2D(datosCanvas)
let canvasManager = new CanvasManager(contextGenerator2D)
let context = <CanvasRenderingContext2D>canvasManager.context
let drawer = new DrawerImage2D(context)
let clearer = new ClearerImage2D(context)
let spritemanager = new SpriteManager(spriteAfor)
let lector = new Speaker()

canvasManager.drawImage(drawer, <HTMLImageElement>img,
    spritemanager.getPosition("avatar"))
canvasManager.drawImage(drawer, <HTMLImageElement>img,
    spritemanager.getPosition("m"))

let time = 0
let pause = 75
botonHablar?.addEventListener("click", () => {
    time = 0
    let texto = (<HTMLInputElement>elementoTexto).value;
    lector.speak(texto)
    texto.split("").forEach(character => {
        if (character !== "" && spritemanager.exist(character)) {
            Util.delay(() => {
                canvasManager.clearImage(clearer)
                canvasManager.drawImage(drawer, <HTMLImageElement>img,
                    spritemanager.getPosition("avatar"))
                canvasManager.drawImage(drawer, <HTMLImageElement>img,
                    spritemanager.getPosition(character))
            }, time)
            time += pause
        }
    });
    time += pause
    Util.delay(() => {
        canvasManager.clearImage(clearer)
        canvasManager.drawImage(drawer, <HTMLImageElement>img,
            spritemanager.getPosition("avatar"))
        canvasManager.drawImage(drawer, <HTMLImageElement>img,
            spritemanager.getPosition("m"))
    }, time)
})

```

## Source file: src/Speaker.ts

{
  "path": "src/Speaker.ts",
  "blob_sha": "1b0d3179ae4330b7b7fbbdd3094fdd1e712ef412",
  "sha256": "79d18bb0e0e46ef98576e4b63c2bf0607cd4a1e58d25233328227dba61045ce2",
  "lines": 12,
  "bytes": 315,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/intercambioneuronal/mouth-animation/blob/f4e21e503cb6ae1ae6ffa79d93262e82624234bf/src/Speaker.ts",
  "function_signature_candidates": []
}

```text

import { ISpeaker } from "./iSpeaker";

export class Speaker implements ISpeaker{
    _synth:SpeechSynthesis
    constructor(){
        this._synth=window.speechSynthesis
    }
    speak(text: string): void {
        const utterThis = new SpeechSynthesisUtterance(text);
        this._synth.speak(utterThis)
    }
}

```

## Source file: src/iCanvas.ts

{
  "path": "src/iCanvas.ts",
  "blob_sha": "340ac2bebbb28a3bf85839e3375cb72bb17bfa59",
  "sha256": "8938a3381dbdbee11c14753e1856681f0a48fad4982e91c362d585291cc18bb6",
  "lines": 4,
  "bytes": 64,
  "selection_score": 0,
  "keyword_line_counts": {
    "face": 1
  },
  "url": "https://github.com/intercambioneuronal/mouth-animation/blob/f4e21e503cb6ae1ae6ffa79d93262e82624234bf/src/iCanvas.ts",
  "function_signature_candidates": []
}

```text

export interface ICanvas{
    idCanvas:string,
    type:string
}

```
