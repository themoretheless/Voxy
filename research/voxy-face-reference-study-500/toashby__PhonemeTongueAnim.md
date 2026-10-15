# Facial rendering source scan: toashby/PhonemeTongueAnim

Repository: https://github.com/toashby/PhonemeTongueAnim

Pinned source revision: e11b73fd0f31c28c978fc336b8e09c5584118da5

Discovery topic: tongue animation

Description: Phoneme Based Tongue Animation

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: PIScript.py

{
  "path": "PIScript.py",
  "blob_sha": "24fe75798dff6857eb70094f48aa0a0cc65b3cf2",
  "sha256": "9669c9fd52590299dbb0a438c5438e1525adfb9f345f6d3310e92ede54956bab",
  "lines": 144,
  "bytes": 6878,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/toashby/PhonemeTongueAnim/blob/e11b73fd0f31c28c978fc336b8e09c5584118da5/PIScript.py",
  "function_signature_candidates": []
}

```text

#python PI script

import requests
#r = requests.get('http://127.0.0.1:8765')
#r.json()

params = (
    ('async', 'false'),
)

files = {
    'audio': ('audio.mp3', open('ttsaudio.mp3', 'rb')),
    'transcript': ('words.txt', open('words.txt', 'rb')),
}

response = requests.post('http://127.0.0.1:8765/transcriptions', params=params, files=files)
data = response.json()
print(response.json())

f= open("outScript.txt","w+") #create file to write script to


for phoneList in data['words']:
    i = 0

    oldWordPos = 0 #These keep track of positions in the current word

    while i < len(phoneList['phones']):
        #print(phoneList['phones'][i]["duration"])
        duration = phoneList['phones'][i]["duration"]
        start = phoneList['start']
        #print(phoneList['phones'][i]["phone"])

        f.write("currentTime %f ;\r\n" % ((phoneList['start'] +oldWordPos)*24))

        oldWordPos = oldWordPos + phoneList['phones'][i]["duration"]
        #make script here
        #end time - startTime == sum of all durations on phones in that word
        #Therefore phone start == startTime + all previous phones in that word

        rest = True

        if "dh_" in str(phoneList['phones'][i]):
            rest = False
            print("found dh_ - posed")
         
            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name dh character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "k_" in str(phoneList['phones'][i]) or "'g_" in str(phoneList['phones'][i]):
            rest = False
            print("found k_ / g_ - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name k character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "zh_" in str(phoneList['phones'][i]) or "sh_" in str(phoneList['phones'][i]):
            rest = False
            print("found zh_ / Sh - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name Ezh character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "s_" in str(phoneList['phones'][i]) or "z_" in str(phoneList['phones'][i]):
            rest = False
            print("found s_ / z_ - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name s character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "jh_" in str(phoneList['phones'][i]) or "ch_" in str(phoneList['phones'][i]):
            rest = False
            print("found jh_/ ch_ - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name Dezh character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "t_" in str(phoneList['phones'][i]) or "n_" in str(phoneList['phones'][i]) or "d_" in str(phoneList['phones'][i]):
            rest = False
            print("found t_ / n_ / d_ - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name t character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "th_" in str(phoneList['phones'][i]):
            rest = False
            print("found th_ - posed")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name Theta character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if "l_" in str(phoneList['phones'][i]): #separate because this might be different
            rest = False
            print("found l_ - posed - same as t MAYBE")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name t character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")

        if rest == True: 
            print("Rest")

            f.write("select -r joint3 joint1 joint2 ;\r\n")
            f.write("pose -apply -name poseRest character1 ;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2.translate;\r\n")
            f.write("setKeyframe -breakdown 0 |joint1|joint2|joint3.translate;\r\n")
            f.write("setKeyframe -breakdown 0 -hierarchy none -controlPoints 0 -shape 0 {\"joint1\", \"joint2\", \"joint3\"};\r\n")


        i = i + 1

f.close() 

```
