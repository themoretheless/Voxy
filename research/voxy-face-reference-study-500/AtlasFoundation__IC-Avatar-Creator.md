# Facial rendering source scan: AtlasFoundation/IC-Avatar-Creator

Repository: https://github.com/AtlasFoundation/IC-Avatar-Creator

Pinned source revision: c03087867e59d3787cb23fddbeff2fa03ce6e548

Discovery topic: character creator 3D

Description: A React component to add a 3D character creator to your dapp

Reported license: MIT

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"rig": [".prettierignore"], "face": ["src/ic/interfaces/motokoStorage/src/declarations/storage/index.js", "src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did", "src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did.d.ts", "src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did.js", "src/ic/interfaces/motokoStorage/src/declarations/storage/storage.old.did", "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/assetstorage.did", "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/index.js", "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/storage_assets.did", "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/storage_assets.did.d.ts", "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/storage_assets.did.js", "src/ic/interfaces/motokoStorage/src/storage/main.mo", "src/ic/interfaces/motokoStorage/src/storage/types.mo", "src/ic/interfaces/motokoStorage/src/storage_assets/assets/favicon.ico", "src/ic/interfaces/motokoStorage/src/storage_assets/assets/logo.png", "src/ic/interfaces/motokoStorage/src/storage_assets/assets/main.css", "src/ic/interfaces/motokoStorage/src/storage_assets/assets/sample-asset.txt", "src/ic/interfaces/motokoStorage/src/storage_assets/src/index.html", "src/ic/interfaces/motokoStorage/src/storage_assets/src/index.js"]}

## Source file: src/ic/interfaces/motokoStorage/src/storage_assets/src/index.js

{
  "path": "src/ic/interfaces/motokoStorage/src/storage_assets/src/index.js",
  "blob_sha": "d4aa064152a4a697fa3cb30d010d9a235254b92b",
  "sha256": "2059f509538730e32060a94b6da570f726d1f7c75906f924c45685790ebe5b99",
  "lines": 71,
  "bytes": 1617,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/AtlasFoundation/IC-Avatar-Creator/blob/c03087867e59d3787cb23fddbeff2fa03ce6e548/src/ic/interfaces/motokoStorage/src/storage_assets/src/index.js",
  "function_signature_candidates": []
}

```text

import { storage } from "../../declarations/storage";

let file;

const uploadChunk = async ({batch_name, chunk}) => storage.create_chunk({
    batch_name,
    content: [...new Uint8Array(await chunk.arrayBuffer())],
})

const upload = async () => {

    if (!file) {
        alert("File not found");
        return;
    }

    console.log("start upload");

    const batch_name = file.name;
    const chunks = [];
    const chunkSize = 1500000

    for (let start = 0; start < file.size; start += chunkSize) {
      const chunk = file.slice(start, start + chunkSize);

      chunks.push(uploadChunk({
        batch_name,
        chunk
      }))
    }

    const chunkIds = await Promise.all(chunks);

    console.log("chunkIds", chunkIds);

    await storage.commit_batch({
        batch_name,
        chunk_ids: chunkIds.map(({chunk_id}) => chunk_id),
        content_type: file.type
    })

    console.log("upload finished");

    loadImage(batch_name);
}

const loadImage = (batch_name) => {

  if (!batch_name) {
    return;
  }

  const newImage = document.createElement("img");
  newImage.src = `http://piwdi-uyaaa-aaaam-qaojq-cai.raw.ic0.app/assets/${batch_name}`;

  const img = document.querySelector('section:last-of-type img');
  img?.parentElement.removeChild(img);

  const section = document.querySelector('section:last-of-type'); 
  section?.appendChild(newImage);
}

const input = document.querySelector('input');
input.addEventListener('change', (e) => {
  file = event.target.files[0];
});

const btnUpload = document.querySelector('button.upload');
btnUpload.addEventListener('click', upload);




```

## Source file: src/ic/interfaces/motokoStorage/src/declarations/storage/index.js

{
  "path": "src/ic/interfaces/motokoStorage/src/declarations/storage/index.js",
  "blob_sha": "acbc418fb887b26da977c9e88ff944672b63a675",
  "sha256": "8515deca8dcf1db421318bf612ead136a8534eb2a11eacdb63853f146972a334",
  "lines": 20,
  "bytes": 760,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/AtlasFoundation/IC-Avatar-Creator/blob/c03087867e59d3787cb23fddbeff2fa03ce6e548/src/ic/interfaces/motokoStorage/src/declarations/storage/index.js",
  "function_signature_candidates": []
}

```text

import { Actor, HttpAgent } from "@dfinity/agent";

// Imports and re-exports candid interface
import { idlFactory } from './storage.did.js';
export { idlFactory } from './storage.did.js';

/**
 * 
 * @param {string | import("@dfinity/principal").Principal} canisterId Canister ID of Agent
 * @param {{agentOptions?: import("@dfinity/agent").HttpAgentOptions; actorOptions?: import("@dfinity/agent").ActorConfig}} [options]
 * @return {import("@dfinity/agent").ActorSubclass<import("./storage.did.js")._SERVICE>}
 */
 export const createActor = (canisterId, agent, options) => {  
  // Creates an actor with using the candid interface and the HttpAgent
  return Actor.createActor(idlFactory, {
    agent,
    canisterId,
    ...options?.actorOptions,
  });
};


```

## Source file: src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did.js

{
  "path": "src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did.js",
  "blob_sha": "75a52f0c9bbfb3fed9778dfc832e6bd5bdbecbc9",
  "sha256": "d5d11cfc69300405fee61d97220d9bb874798aa298a26503e96cdd6240168a9e",
  "lines": 61,
  "bytes": 1848,
  "selection_score": 3,
  "keyword_line_counts": {},
  "url": "https://github.com/AtlasFoundation/IC-Avatar-Creator/blob/c03087867e59d3787cb23fddbeff2fa03ce6e548/src/ic/interfaces/motokoStorage/src/declarations/storage/storage.did.js",
  "function_signature_candidates": []
}

```text

export const idlFactory = ({ IDL }) => {
    const Chunk = IDL.Record({
      'content' : IDL.Vec(IDL.Nat8),
      'batch_name' : IDL.Text,
    });
    const HeaderField = IDL.Tuple(IDL.Text, IDL.Text);
    const HttpRequest = IDL.Record({
      'url' : IDL.Text,
      'method' : IDL.Text,
      'body' : IDL.Vec(IDL.Nat8),
      'headers' : IDL.Vec(HeaderField),
    });
    const StreamingCallbackToken = IDL.Record({
      'key' : IDL.Text,
      'index' : IDL.Nat,
      'content_encoding' : IDL.Text,
    });
    const StreamingStrategy = IDL.Variant({
      'Callback' : IDL.Record({
        'token' : StreamingCallbackToken,
        'callback' : IDL.Func([], [], []),
      }),
    });
    const HttpResponse = IDL.Record({
      'body' : IDL.Vec(IDL.Nat8),
      'headers' : IDL.Vec(HeaderField),
      'streaming_strategy' : IDL.Opt(StreamingStrategy),
      'status_code' : IDL.Nat16,
    });
    const StreamingCallbackHttpResponse = IDL.Record({
      'body' : IDL.Vec(IDL.Nat8),
      'headers' : IDL.Vec(HeaderField),
      'streaming_strategy' : IDL.Opt(StreamingStrategy),
      'status_code' : IDL.Nat16,
    });
    return IDL.Service({
      'commit_batch' : IDL.Func(
          [
            IDL.Record({
              'batch_name' : IDL.Text,
              'content_type' : IDL.Text,
              'chunk_ids' : IDL.Vec(IDL.Nat),
            }),
          ],
          [],
          [],
        ),
      'create_chunk' : IDL.Func(
          [Chunk],
          [IDL.Record({ 'chunk_id' : IDL.Nat })],
          [],
        ),
      'http_request' : IDL.Func([HttpRequest], [HttpResponse], ['query']),
      'http_request_streaming_callback' : IDL.Func(
          [StreamingCallbackToken],
          [StreamingCallbackHttpResponse],
          ['query'],
        ),
    });
  };
  export const init = ({ IDL }) => { return []; };

```

## Source file: src/ic/interfaces/motokoStorage/src/declarations/storage_assets/index.js

{
  "path": "src/ic/interfaces/motokoStorage/src/declarations/storage_assets/index.js",
  "blob_sha": "bb4a0c034b24d592e111a72026a99e1feca1578c",
  "sha256": "f4db8bd87a07b370cbd70ed77fd3fb7b75f774b965b865a0df3d5bbabaa5e527",
  "lines": 38,
  "bytes": 1472,
  "selection_score": 3,
  "keyword_line_counts": {
    "face": 2
  },
  "url": "https://github.com/AtlasFoundation/IC-Avatar-Creator/blob/c03087867e59d3787cb23fddbeff2fa03ce6e548/src/ic/interfaces/motokoStorage/src/declarations/storage_assets/index.js",
  "function_signature_candidates": []
}

```text

import { Actor, HttpAgent } from "@dfinity/agent";

// Imports and re-exports candid interface
import { idlFactory } from './storage_assets.did.js';
export { idlFactory } from './storage_assets.did.js';
// CANISTER_ID is replaced by webpack based on node environment
export const canisterId = process.env.STORAGE_ASSETS_CANISTER_ID;

/**
 * 
 * @param {string | import("@dfinity/principal").Principal} canisterId Canister ID of Agent
 * @param {{agentOptions?: import("@dfinity/agent").HttpAgentOptions; actorOptions?: import("@dfinity/agent").ActorConfig}} [options]
 * @return {import("@dfinity/agent").ActorSubclass<import("./storage_assets.did.js")._SERVICE>}
 */
 export const createActor = (canisterId, options) => {
  const agent = new HttpAgent({ ...options?.agentOptions });
  
  // Fetch root key for certificate validation during development
  if(process.env.NODE_ENV !== "production") {
    agent.fetchRootKey().catch(err=>{
      console.warn("Unable to fetch root key. Check to ensure that your local replica is running");
      console.error(err);
    });
  }

  // Creates an actor with using the candid interface and the HttpAgent
  return Actor.createActor(idlFactory, {
    agent,
    canisterId,
    ...options?.actorOptions,
  });
};
  
/**
 * A ready-to-use agent for the storage_assets canister
 * @type {import("@dfinity/agent").ActorSubclass<import("./storage_assets.did.js")._SERVICE>}
 */
 export const storage_assets = createActor(canisterId);


```
