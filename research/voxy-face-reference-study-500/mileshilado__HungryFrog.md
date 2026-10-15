# Facial rendering source scan: mileshilado/HungryFrog

Repository: https://github.com/mileshilado/HungryFrog

Pinned source revision: f4697b22c970a05eee319d7cf4716670c7bc009b

Discovery topic: tongue animation

Description: frog-themed Pygame where players control a frog to catch as many fleas as possible by extending the frog's tongue. The game features smooth animations, audio effects, and progressively faster gameplay as the player levels up.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: main.py

{
  "path": "main.py",
  "blob_sha": "53f61a48fe5c7d759e5b745831c184fbe0719267",
  "sha256": "77c78187fa383f16d3a5df90085d6bd378261f5980e11975543a6231db5528e6",
  "lines": 357,
  "bytes": 14048,
  "selection_score": 0,
  "keyword_line_counts": {
    "mouth": 1,
    "tongue": 56,
    "rig": 17,
    "face": 1,
    "lip": 1
  },
  "url": "https://github.com/mileshilado/HungryFrog/blob/f4697b22c970a05eee319d7cf4716670c7bc009b/main.py",
  "function_signature_candidates": [
    {
      "line": 21,
      "text": "def draw_stats(remaining_time,score,level):"
    },
    {
      "line": 59,
      "text": "def mirror_images(images):"
    },
    {
      "line": 63,
      "text": "def __init__(self, filename):"
    },
    {
      "line": 69,
      "text": "def image_at(self, rectangle, colorkey = None):"
    },
    {
      "line": 80,
      "text": "def images_at(self, rects, colorkey = None):"
    },
    {
      "line": 84,
      "text": "def load_strip(self, rect, image_count, colorkey = None):"
    },
    {
      "line": 95,
      "text": "def scale_images(images, scale_factor):"
    },
    {
      "line": 110,
      "text": "def draw_tongue(player, tongue_length, tongue_target_pos,directionfacing):"
    },
    {
      "line": 140,
      "text": "def draw(player,fleas,current_frame,directionfacing,remaining_time,score,level):"
    },
    {
      "line": 163,
      "text": "def main():"
    }
  ]
}

```text

import pygame
import time
import random
import math

pygame.font.init()
font = pygame.font.SysFont('roboto', 40)
pygame.mixer.init()
pygame.mixer.music.set_volume(0.4)
hitsound1 = pygame.mixer.Sound('sound/hit1.mp3')  # Replace with your sound file
hitsound2 = pygame.mixer.Sound('sound/hit2.mp3')
pygame.mixer.music.load('sound/weezer.mp3')  # Replace with your MP3 file
pygame.mixer.music.play(-1)  # -1 makes the music loop indefinitely 

WIDTH,HEIGHT = 720,960
WIN = pygame.display.set_mode((WIDTH,HEIGHT))
pygame.display.set_caption("jpegfrog")

background = pygame.image.load("forest.png")

def draw_stats(remaining_time,score,level):
    # Convert remaining time to seconds
    seconds = math.ceil(remaining_time/1000)
    timer_text = font.render(f"Time: {seconds}", True, (255, 255, 255))  # White color for the text
    timer_text_outline = font.render(f"Time: {seconds}", True, (0, 0, 0))  # Black color for the outline
    score_text = font.render(f"Score: {score}", True, (255, 255, 255))  # White color for the text
    score_text_outline = font.render(f"Score: {score}", True, (0, 0, 0))  # Black color for the outline
    WIN.blit(timer_text_outline, (12,12))
    WIN.blit(timer_text, (10, 10))
    WIN.blit(score_text_outline, (12,42))
    WIN.blit(score_text, (10, 40))

    end_text = font.render(f"YIPEEE!!! YOU GOT {score} FLIES!!!", True, (255, 255, 255))  # White color for the text
    if seconds <= 0:
        WIN.blit(end_text, (12.5, (HEIGHT/2)-200))

    

grass = 508

tongue_active = False  # Is the tongue extending or not
tongue_length = None  # Current length of the tongue
tongue_max_length = None  # Maximum length the tongue can reach
tongue_speed = None  # How fast the tongue grows
tongue_target_pos = (0, 0)  # Target position (mouse cursor)
tongue_cooldown = 0
tongue_retracting = False


#flea VARIABLES
flea_WIDTH = 5
flea_HEIGHT = 5
flea_VEL = 4

#PLAYER VARIABLES
PLAYER_WIDTH,PLAYER_HEIGHT = 96,140
global PLAYER_VEL

def mirror_images(images):
    return [pygame.transform.flip(image, True, False) for image in images]

class spritesheet(object):
    def __init__(self, filename):
        try:
            self.sheet = pygame.image.load(filename).convert()
        except pygame.error as message:
            exit("oops")
    # Load a specific image from a specific rectangle
    def image_at(self, rectangle, colorkey = None):
        "Loads image from x,y,x+offset,y+offset"
        rect = pygame.Rect(rectangle)
        image = pygame.Surface(rect.size).convert()
        image.blit(self.sheet, (0, 0), rect)
        if colorkey != None:
            if colorkey == -1:
                colorkey = image.get_at((0,0))
            image.set_colorkey(colorkey, pygame.RLEACCEL)
        return image
    # Load a whole bunch of images and return them as a list
    def images_at(self, rects, colorkey = None):
        "Loads multiple images, supply a list of coordinates" 
        return [self.image_at(rect, colorkey) for rect in rects]
    # Load a whole strip of images
    def load_strip(self, rect, image_count, colorkey = None):
        "Loads a strip of images and returns them as a list"
        tups = [(rect[0]+rect[2]*x, rect[1], rect[2], rect[3])
                for x in range(image_count)]
        return self.images_at(tups, colorkey)


#LOAD FROG SPRITES
idlesprites = spritesheet('frog/PNG/froglet_frog_green_sheet_idle.png')
walksprites = spritesheet('frog/PNG/froglet_frog_green_sheet_walk.png')

def scale_images(images, scale_factor):
    return [pygame.transform.scale(image, (int(image.get_width() * scale_factor), int(image.get_height() * scale_factor))) for image in images]

#LOAD IDLE FRAMES
idle_frames = idlesprites.images_at([(0, 0, 16, 20),(16, 0, 16, 20),(32, 0, 16, 20),(48, 0, 16, 20),(64, 0, 16, 20),(80, 0, 16, 20),(96, 0, 16, 20)], colorkey=(0,0,0))
scale_factor = 8
right_idle_frames = scale_images(idle_frames, scale_factor)
left_idle_frames = mirror_images(right_idle_frames)
#LOAD WALKING FRAMES
walk_frames = walksprites.images_at([(0, 0, 16, 20),(16, 0, 16, 20),(32, 0, 16, 20),(48, 0, 16, 20),(64, 0, 16, 20),(80, 0, 16, 20),(96, 0, 16, 20)], colorkey=(0,0,0))
scale_factor = 8
right_walk_frames = scale_images(walk_frames, scale_factor)
left_walk_frames = mirror_images(right_walk_frames)


def draw_tongue(player, tongue_length, tongue_target_pos,directionfacing):
    # Get the center of the player's mouth
    start_pos = (player.x + PLAYER_WIDTH // 2, (player.y + PLAYER_HEIGHT // 2)+25) #facing left tongue is good

    #facing right
    if directionfacing == "right":
        start_pos = (player.x + PLAYER_WIDTH - 20 // 2, (player.y + PLAYER_HEIGHT // 2)+25) #facing left tongue is good
    
    # Calculate the direction vector to the mouse cursor
    dir_x, dir_y = tongue_target_pos[0] - start_pos[0], tongue_target_pos[1] - start_pos[1]
    distance = math.sqrt(dir_x**2 + dir_y**2)

    # Normalize direction vector
    if distance != 0:
        dir_x /= distance
        dir_y /= distance

    # Calculate the tongue's current end position
    end_pos = (start_pos[0] + dir_x * tongue_length, start_pos[1] + dir_y * tongue_length)

    pygame.draw.line(WIN, (255, 0, 0), start_pos, end_pos, 5)  # Draw a red line as the tongue
    
    # Create a bounding box for the tongue
    tongue_rect = pygame.Rect(min(start_pos[0], end_pos[0]), min(start_pos[1], end_pos[1]),
                              abs(end_pos[0] - start_pos[0]), abs(end_pos[1] - start_pos[1]))
    return tongue_rect




def draw(player,fleas,current_frame,directionfacing,remaining_time,score,level):
    WIN.blit(background,(0,0))

    WIN.blit(current_frame,(player.x,player.y))

    for flea in fleas:
        pygame.draw.rect(WIN,"black",flea)
    
    if tongue_active:
        tongue_rect = draw_tongue(player, tongue_length,tongue_target_pos,directionfacing)  # Draw the tongue
    
    level_pos = ((player.x + PLAYER_WIDTH // 2)-30, (player.y + PLAYER_HEIGHT // 2)+60)
    level_text = font.render(f"Level: {level}", True, (255, 255, 255))  # White color for the text
    outline_text = font.render(f"Level: {level}", True, (0, 0, 0))  # Black color for the outline

    WIN.blit(outline_text, (level_pos[0] + 2, level_pos[1] + 2))  # Bottom-right
    WIN.blit(level_text, (level_pos))

    draw_stats(remaining_time,score,level)
    pygame.display.update()



def main():
    global tongue_active, tongue_length, tongue_target_pos, killcount  # Ensure we modify these global variables
    global tongue_max_length, PLAYER_VEL, tongue_speed
    run = True
    PLAYER_VEL = 4

    game_duration = 60 * 1000  # Game lasts for 60 seconds (in milliseconds)
    start_time = pygame.time.get_ticks()  # Capture the time when the game starts


    player = pygame.Rect(200,HEIGHT-PLAYER_HEIGHT,PLAYER_WIDTH,PLAYER_HEIGHT)

    clock = pygame.time.Clock()

    #flea variables
    flea_add_increment = 2000
    flea_count = 0
    fleas = []
    global flea_VEL
    #player variables
    level = 1
    directionfacing = "right"
    walkstate = False
    hit = False
    killcount = 0
    score = 0
    killincrement = 5
    #TONGUE VARIABLES
    tongue_active = False  # Is the tongue extending or not
    tongue_length = 0  # Current length of the tongue
    tongue_max_length = 100  # Maximum length the tongue can reach
    tongue_speed = 5  # How fast the tongue grows
    tongue_retracting = False
    tongue_cooldown = 0
    cooldown_rate = 6

    # Animation control
    frame_index = 0  # Current frame index for idle animation
    frame_delay = 100  # Time delay between frames (in milliseconds)
    last_frame_time = pygame.time.get_ticks()

    while run:
        flea_count += clock.tick(60)

        #game timer
        current_time = pygame.time.get_ticks()
        elapsed_time = current_time - start_time
        remaining_time = max(0, game_duration - elapsed_time)


        if flea_count > flea_add_increment:
            for _ in range(random.randint(0, 10)):  # Add random fleas
                flea_x = WIDTH + flea_WIDTH + _*10  # Start just outside the right edge
                flea_y = random.randint(-flea_HEIGHT, 700)  # Random vertical position
                flea = pygame.Rect(flea_x, flea_y, flea_WIDTH, flea_HEIGHT)
                fleas.append(flea)

            flea_add_increment = max(200, flea_add_increment - 100)
            flea_count = 0

        for flea in fleas:
            flea.x -= flea_VEL  # Move fleas leftwards
            direction = random.randint(1,4)
            if direction == 1:
                flea.x -= flea_VEL
            elif direction == 2:
                flea.y += flea_VEL
            elif direction == 3:
                flea.y -= flea_VEL
            elif direction == 3:
                flea.x += flea_VEL
            if flea.x < -flea_WIDTH:  # Remove fleas if they go off-screen on the left
                fleas.remove(flea)


        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                run = False
                break
            elif event.type == pygame.MOUSEBUTTONDOWN and event.button == 1:  # Left click
                if not tongue_active and not tongue_retracting and tongue_cooldown <= 0:
                    tongue_active = True
                    tongue_length = 0  # Reset tongue length
                    tongue_target_pos = pygame.mouse.get_pos()
                
        current_time = pygame.time.get_ticks()
        if current_time - last_frame_time > frame_delay:
            frame_index = (frame_index + 1) % len(idle_frames)  # Loop through frames
            last_frame_time = current_time
        
        # Tongue logic (extension and retraction)
        if tongue_active:
            if not tongue_retracting:
                # Tongue extends
                tongue_length += tongue_speed
                if tongue_length >= tongue_max_length:
                    tongue_length = tongue_max_length
                    tongue_retracting = True  # Start retracting the tongue
            else:
                # Tongue retracts
                tongue_length -= tongue_speed
                if tongue_length <= 0:
                    tongue_length = 0
                    tongue_active = False
                    tongue_retracting = False  # Set a cooldown period (e.g., 30 frames)
                    tongue_cooldown = cooldown_rate

        # Handle cooldown timer
        if tongue_cooldown > 0:
            tongue_cooldown -= 1
        
        #wasd movement code
        keys = pygame.key.get_pressed()
        walkstate = True
        if keys[pygame.K_a] and player.x - PLAYER_VEL >= 0: #left
            player.x -= PLAYER_VEL
            directionfacing = "left"
            if keys[pygame.K_s] and player.y + PLAYER_VEL + PLAYER_HEIGHT < HEIGHT + 15: #left down
                player.y += PLAYER_VEL
            if keys[pygame.K_w] and player.y - PLAYER_VEL + PLAYER_HEIGHT > grass: #left down
                player.y -= PLAYER_VEL
        elif keys[pygame.K_d] and player.x + PLAYER_WIDTH + PLAYER_VEL <= WIDTH: #right
            player.x += PLAYER_VEL
            directionfacing = "right"
            if keys[pygame.K_s] and player.y + PLAYER_VEL + PLAYER_HEIGHT < HEIGHT + 15: #down right
                player.y += PLAYER_VEL
            if keys[pygame.K_w] and player.y - PLAYER_VEL + PLAYER_HEIGHT > grass: #up right
                player.y -= PLAYER_VEL
        elif keys[pygame.K_w] and player.y - PLAYER_VEL + PLAYER_HEIGHT > grass: #up
            player.y -= PLAYER_VEL
        elif keys[pygame.K_s] and player.y + PLAYER_VEL + PLAYER_HEIGHT < HEIGHT + 15: #down
            player.y += PLAYER_VEL
        else:
            walkstate = False
        

        current_time = pygame.time.get_ticks()
        if current_time - last_frame_time > frame_delay:
            frame_index = (frame_index + 1) % len(idle_frames)  # Loop through frames
            last_frame_time = current_time

        
        
        #ANIMATION SELECTOR
        #idling
        if walkstate == False:
            if directionfacing == "right":
                current_frame = right_idle_frames[frame_index]  # Get the current frame to display
            elif directionfacing == "left":
                current_frame = left_idle_frames[frame_index]  # Get the current frame to display
        elif walkstate == True:
            if directionfacing == "right":
                current_frame = right_walk_frames[frame_index]  # Get the current frame to display
            elif directionfacing == "left":
                current_frame = left_walk_frames[frame_index]  # Get the current frame to display
        draw(player,fleas,current_frame,directionfacing,remaining_time,score,level)

        
        tongue_rect = None
        if tongue_active:
            tongue_rect = draw_tongue(player, tongue_length, tongue_target_pos,directionfacing)
        
        # Check for collisions
        if tongue_rect:
            remaining_fleas = []
            for flea in fleas:
                if tongue_rect.colliderect(flea) and math.ceil(remaining_time/1000) > 0:
                    # Randomly choose one of the two sound effects to play
                    if random.randint(0, 1) == 0:
                        hitsound1.play()  # Play the first sound effect
                    else:
                        hitsound2.play()  # Play the second sound effect
                    # Don't add the flea to remaining_fleas (effectively removing it)
                    killcount += 1
                    score+=1
                else:
                    remaining_fleas.append(flea)  # Only add fleas that are not hit

            fleas = remaining_fleas
            #LEVEL UP
            if(killcount % killincrement == 0 and killcount > 0):
                tongue_max_length+=60
                PLAYER_VEL +=4
                tongue_speed +=10
                killcount = 0
                level += 1
                cooldown_rate -= 2
                killincrement += 5
    
    pygame.quit()



if __name__ == "__main__":
    main()

```

## Source file: refactoring/game.py

{
  "path": "refactoring/game.py",
  "blob_sha": "ee7e1572ed139a734c2c88406b51547aca394ef0",
  "sha256": "19239d1b44d9bc85a744b980d6ada60bb117b62e9d6ccc1058f712a4b70012a5",
  "lines": 1,
  "bytes": 38,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mileshilado/HungryFrog/blob/f4697b22c970a05eee319d7cf4716670c7bc009b/refactoring/game.py",
  "function_signature_candidates": []
}

```text

from state_machine import StateMachine

```

## Source file: refactoring/main.py

{
  "path": "refactoring/main.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mileshilado/HungryFrog/blob/f4697b22c970a05eee319d7cf4716670c7bc009b/refactoring/main.py",
  "function_signature_candidates": []
}

```text



```

## Source file: refactoring/pause.py

{
  "path": "refactoring/pause.py",
  "blob_sha": "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
  "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "lines": 0,
  "bytes": 0,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/mileshilado/HungryFrog/blob/f4697b22c970a05eee319d7cf4716670c7bc009b/refactoring/pause.py",
  "function_signature_candidates": []
}

```text



```
