# Facial rendering source scan: zhoppock/Attack-Frog

Repository: https://github.com/zhoppock/Attack-Frog

Pinned source revision: a1080762d08763d06dd072f3fa9766d0f5082165

Discovery topic: tongue animation

Description: Sprite animation of a frog attacking with its tongue.  Press a key to make it happen. (Assets are not mine)

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {"tongue": ["Assets/Sounds/Yoshi_tongue_sound.mp3"]}

## Source file: frog_test.py

{
  "path": "frog_test.py",
  "blob_sha": "18d1fc9ac8e791c5b623b8a16d2918375d985c74",
  "sha256": "c84999f5f3873d6f3ff2adef4d250ed14aa6cb6de83bae8da9c79c3335845154",
  "lines": 62,
  "bytes": 3286,
  "selection_score": -2,
  "keyword_line_counts": {
    "tongue": 2,
    "lip": 1
  },
  "url": "https://github.com/zhoppock/Attack-Frog/blob/a1080762d08763d06dd072f3fa9766d0f5082165/frog_test.py",
  "function_signature_candidates": [
    {
      "line": 7,
      "text": "def __init__(self, x_position, y_position):"
    },
    {
      "line": 19,
      "text": "def move(self): # gives the below functions permission to activate"
    },
    {
      "line": 23,
      "text": "def update(self, speed): # makes sure the animation goes smoothly"
    },
    {
      "line": 45,
      "text": "def main():"
    }
  ]
}

```text

from turtle import window_height
import pygame, sys # import the libraries for pygame and sys at the same time
pygame.init() # activate pygame usefulness
clock = pygame.time.Clock() # set up clock for framerate

class Frog(pygame.sprite.Sprite): # sprite class for the frog frames
    def __init__(self, x_position, y_position):
        super().__init__()
        self.frog_frames = []
        self.is_moving = False
        for frame in range(0, 10):
            load_frame = pygame.image.load('./Assets/Images/attack_' + str(frame + 1) + '.png')
            self.frog_frames.append(pygame.transform.scale(load_frame, (300, 143)))
        self.current_frame = 0 # this will increment by 1 for each frame in the loop below
        self.image = self.frog_frames[self.current_frame] # displays the current sprite frame based on list element
        self.rect = self.image.get_rect()
        self.rect.topleft = [x_position, y_position]
    
    def move(self): # gives the below functions permission to activate
        self.is_moving = True
        self.noise = True

    def update(self, speed): # makes sure the animation goes smoothly
        if self.is_moving == True:
            self.current_frame += speed # goes through each frame of animation
            if self.current_frame == 1: # prevents the sound from playing repeatedly if a key is pressed down rapidly
                tongue_sound = pygame.mixer.Sound('./Assets/Sounds/Yoshi_tongue_sound.mp3')
                tongue_sound.play() # play a noise when the frog attacks
            if self.current_frame >= len(self.frog_frames): # resets the animation after each run through
                self.current_frame = 0
                self.is_moving = False # stops a continuous loop to allow a key to be pressed again
            self.image = self.frog_frames[int(self.current_frame)]
        
width_window = 600
height_window = 350
window = pygame.display.set_mode((width_window, height_window)) # dimensions of the window
load_image = pygame.image.load('./Assets/Images/rainforest_image.jpg') # load in an image to be the background
background_image = pygame.transform.scale(load_image, (width_window, height_window)) # resize the image to fit the window
pygame.display.set_caption("Attack Frog") # name of the window

frog_sprites = pygame.sprite.Group() # group that takes in all the frames for animation
frog = Frog(90, 140)
frog_sprites.add(frog)

def main():
    pygame.mixer.music.load('./Assets/Sounds/BirdsSporadicFrogs CM020101.mp3') # loads in some ambience
    pygame.mixer.music.play(-1) # the -1 tells the program to loop the ambience indefinitely
    while True:
        for event in pygame.event.get():
            if event.type == pygame.QUIT: # if you close the window, the program closes down pygame and sys
                pygame.quit()
                sys.exit()
            if event.type == pygame.KEYDOWN: # press any key to make the frog attack
                frog.move()

        window.blit(background_image, (0, 0)) # puts the background image in the game window, starting from 0 0 xy position
        frog_sprites.draw(window)
        frog_sprites.update(0.2)
        pygame.display.flip()
        clock.tick(60) # framerate

main()


```
