# Facial rendering source scan: abdullahshaikh-0078/Snake-Game

Repository: https://github.com/abdullahshaikh-0078/Snake-Game

Pinned source revision: 52bf69ce776e0c7e749b1caff201f784355c7a15

Discovery topic: tongue animation

Description: 🐍 Ultimate Snake Game is a modern twist on the classic Snake, built with Python and Pygame. It features 3 levels — normal, faster snake, and moving obstacles — along with sound effects, custom backgrounds, and a snake design with eyes, tail, and tongue animation.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: UltimateSnakeGame.py

{
  "path": "UltimateSnakeGame.py",
  "blob_sha": "53fd776f7beda8365ca2ce9f17e93630f51a10fc",
  "sha256": "2fd1c1d1bca75267ab342277805af9ee9569625a450a1eb8e8a2a10f15d11b01",
  "lines": 305,
  "bytes": 10783,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 1,
    "tongue": 10,
    "rig": 1,
    "lip": 5
  },
  "url": "https://github.com/abdullahshaikh-0078/Snake-Game/blob/52bf69ce776e0c7e749b1caff201f784355c7a15/UltimateSnakeGame.py",
  "function_signature_candidates": [
    {
      "line": 59,
      "text": "def show_instructions():"
    },
    {
      "line": 78,
      "text": "def show_levels():"
    },
    {
      "line": 90,
      "text": "def init_snake():"
    },
    {
      "line": 95,
      "text": "def spawn_food():"
    },
    {
      "line": 100,
      "text": "def spawn_obstacles():"
    },
    {
      "line": 110,
      "text": "def move_obstacles():"
    },
    {
      "line": 120,
      "text": "def move_snake():"
    },
    {
      "line": 158,
      "text": "def draw_snake():"
    },
    {
      "line": 215,
      "text": "def draw_food():"
    },
    {
      "line": 218,
      "text": "def draw_obstacles():"
    },
    {
      "line": 222,
      "text": "def reset_game():"
    },
    {
      "line": 233,
      "text": "def game_loop():"
    }
  ]
}

```text

import pygame
import random
import sys

# ---------------- CONFIG ----------------
ROWS, COLS = 25, 25
TILE_SIZE = 25
WINDOW_WIDTH, WINDOW_HEIGHT = COLS*TILE_SIZE, ROWS*TILE_SIZE
FPS = 12  # base FPS

# Colors
GREEN = (0, 200, 0)
DARK_GREEN = (0, 150, 0)
RED = (255, 0, 0)
BLACK = (0,0,0)
WHITE = (255,255,255)

# ---------------- INIT ----------------
pygame.init()
pygame.mixer.init()
screen = pygame.display.set_mode((WINDOW_WIDTH, WINDOW_HEIGHT))
pygame.display.set_caption("Ultimate Snake Game")
clock = pygame.time.Clock()
font = pygame.font.SysFont(None, 30)

# ---------------- LOAD BACKGROUNDS ----------------
instructions_bg = pygame.image.load("instructions.png")
instructions_bg = pygame.transform.scale(instructions_bg, (WINDOW_WIDTH, WINDOW_HEIGHT))

levels_bg = pygame.image.load("levels.png")
levels_bg = pygame.transform.scale(levels_bg, (WINDOW_WIDTH, WINDOW_HEIGHT))

game_bg = pygame.image.load("game.png")
game_bg = pygame.transform.scale(game_bg, (WINDOW_WIDTH, WINDOW_HEIGHT))

# ---------------- LOAD SOUNDS ----------------
try:
    eat_sound = pygame.mixer.Sound("eat.wav")
    gameover_sound = pygame.mixer.Sound("gameover.wav")
except:
    eat_sound = None
    gameover_sound = None
    print("⚠️ Sounds not found. Make sure eat.wav & gameover.wav are in the same folder.")

# ---------------- VARIABLES ----------------
snake = []
direction = (1,0)
food = None
score = 0
game_over = False
tongue_toggle = False
level = 1
snake_speed = FPS

# Level 3 obstacles
obstacles = []

# ---------------- FUNCTIONS ----------------
def show_instructions():
    screen.blit(instructions_bg, (0,0))
    lines = [
        "1. Use arrow keys to move.",
        "2. Eat food to grow and gain points.",
        "3. Avoid obstacles and yourself.",
        "4. Snake can pass through walls.",
        "5. Level 1: Normal.",
        "6. Level 2: Faster snake.",
        "7. Level 3: Obstacles appear.",
        "8. Press SPACE to start the game.",
        "9. Press ESC to quit anytime.",
        "10. Press CTRL+R to restart at any time."
    ]
    for idx, line in enumerate(lines):
        text = font.render(line, True, WHITE)
        screen.blit(text, (50, 30 + idx*35))
    pygame.display.flip()

def show_levels():
    screen.blit(levels_bg, (0,0))
    texts = [
        "LEVEL 1 - Normal",
        "LEVEL 2 - Faster",
        "LEVEL 3 - Obstacles"
    ]
    for i, text_line in enumerate(texts):
        text = font.render(text_line, True, WHITE)
        screen.blit(text, (WINDOW_WIDTH//2 - 100, 150 + i*80))
    pygame.display.flip()

def init_snake():
    global snake, direction
    snake = [{'x': TILE_SIZE*5, 'y': TILE_SIZE*5}]
    direction = (1,0)

def spawn_food():
    global food
    food = {'x': random.randint(0, COLS-1)*TILE_SIZE,
            'y': random.randint(0, ROWS-1)*TILE_SIZE}

def spawn_obstacles():
    global obstacles
    max_obs = min(10, 1 + score//10)
    obstacles = []
    for _ in range(max_obs):
        obs = {'x': random.randint(0, COLS-1)*TILE_SIZE,
               'y': random.randint(0, ROWS-1)*TILE_SIZE,
               'dir': random.choice([(1,0), (-1,0), (0,1), (0,-1)])}
        obstacles.append(obs)

def move_obstacles():
    if level !=3: return
    for obs in obstacles:
        dx, dy = obs['dir']
        obs['x'] += dx * (snake_speed//2)
        obs['y'] += dy * (snake_speed//2)
        # Wrap around
        obs['x'] %= WINDOW_WIDTH
        obs['y'] %= WINDOW_HEIGHT

def move_snake():
    global snake, game_over, score, snake_speed
    dx, dy = direction
    new_head = {'x': (snake[0]['x'] + dx*TILE_SIZE)%WINDOW_WIDTH,
                'y': (snake[0]['y'] + dy*TILE_SIZE)%WINDOW_HEIGHT}

    # collision with self
    for segment in snake:
        if new_head['x']==segment['x'] and new_head['y']==segment['y']:
            game_over = True
            if gameover_sound: gameover_sound.play()
            return

    # collision with obstacles (Level 3)
    if level==3:
        for obs in obstacles:
            if new_head['x']==obs['x'] and new_head['y']==obs['y']:
                game_over=True
                if gameover_sound: gameover_sound.play()
                return

    snake.insert(0, new_head)

    # Eating food
    if new_head['x']==food['x'] and new_head['y']==food['y']:
        score +=1
        spawn_food()
        if eat_sound: eat_sound.play()
        # Level 3: reposition obstacles after eating
        if level==3:
            spawn_obstacles()
        # Speed increase for Level 2 & 3 every 3 foods
        if level in [2,3] and score %3==0:
            global snake_speed
            snake_speed +=1
    else:
        snake.pop()

def draw_snake():
    global tongue_toggle
    for i, segment in enumerate(snake):
        rect = pygame.Rect(segment['x'], segment['y'], TILE_SIZE, TILE_SIZE)
        if i==0: # head
            pygame.draw.ellipse(screen, DARK_GREEN, rect)
            dx, dy = direction
            cx, cy = rect.center
            # Eyes
            if dx==1:
                pygame.draw.circle(screen, BLACK, (cx+6, cy-5), 4)
                pygame.draw.circle(screen, BLACK, (cx+6, cy+5), 4)
            elif dx==-1:
                pygame.draw.circle(screen, BLACK, (cx-6, cy-5), 4)
                pygame.draw.circle(screen, BLACK, (cx-6, cy+5), 4)
            elif dy==-1:
                pygame.draw.circle(screen, BLACK, (cx-5, cy-6), 4)
                pygame.draw.circle(screen, BLACK, (cx+5, cy-6), 4)
            elif dy==1:
                pygame.draw.circle(screen, BLACK, (cx-5, cy+6), 4)
                pygame.draw.circle(screen, BLACK, (cx+5, cy+6), 4)
            # Tongue
            if tongue_toggle:
                tongue_len=15
                if dx==1:
                    pygame.draw.line(screen, RED, (cx+TILE_SIZE//2,cy),(cx+TILE_SIZE//2+tongue_len,cy),3)
                elif dx==-1:
                    pygame.draw.line(screen, RED, (cx-TILE_SIZE//2,cy),(cx-TILE_SIZE//2-tongue_len,cy),3)
                elif dy==1:
                    pygame.draw.line(screen, RED, (cx,cy+TILE_SIZE//2),(cx,cy+TILE_SIZE//2+tongue_len),3)
                elif dy==-1:
                    pygame.draw.line(screen, RED, (cx,cy-TILE_SIZE//2),(cx,cy-TILE_SIZE//2-tongue_len),3)
        elif i==len(snake)-1: # tail
            dx_tail = snake[-2]['x'] - segment['x']
            dy_tail = snake[-2]['y'] - segment['y']
            tail_center = (segment['x']+TILE_SIZE//2, segment['y']+TILE_SIZE//2)
            if dx_tail>0:
                points=[(segment['x']+TILE_SIZE,segment['y']),
                        (segment['x']+TILE_SIZE,segment['y']+TILE_SIZE),
                        tail_center]
            elif dx_tail<0:
                points=[(segment['x'],segment['y']),
                        (segment['x'],segment['y']+TILE_SIZE),
                        tail_center]
            elif dy_tail>0:
                points=[(segment['x'],segment['y']+TILE_SIZE),
                        (segment['x']+TILE_SIZE,segment['y']+TILE_SIZE),
                        tail_center]
            else:
                points=[(segment['x'],segment['y']),
                        (segment['x']+TILE_SIZE,segment['y']),
                        tail_center]
            pygame.draw.polygon(screen,DARK_GREEN,points)
        else: # body
            pygame.draw.ellipse(screen,GREEN,rect)
    tongue_toggle = not tongue_toggle

def draw_food():
    pygame.draw.circle(screen, RED, (food['x']+TILE_SIZE//2, food['y']+TILE_SIZE//2), TILE_SIZE//2-2)

def draw_obstacles():
    for obs in obstacles:
        pygame.draw.rect(screen, (255, 165, 0), (obs['x'], obs['y'], TILE_SIZE, TILE_SIZE))

def reset_game():
    global snake, direction, food, game_over, score, snake_speed, obstacles
    snake = [{'x': TILE_SIZE*5,'y':TILE_SIZE*5}]
    direction=(1,0)
    score=0
    game_over=False
    snake_speed=FPS
    spawn_food()
    obstacles=[]

# ---------------- MAIN LOOP ----------------
def game_loop():
    global direction, level, game_over
    waiting_instr=True
    waiting_level=True

    # Instructions page
    while waiting_instr:
        show_instructions()
        for event in pygame.event.get():
            if event.type==pygame.QUIT:
                pygame.quit(); sys.exit()
            if event.type==pygame.KEYDOWN:
                if event.key==pygame.K_SPACE:
                    waiting_instr=False

    # Level selection page
    while waiting_level:
        show_levels()
        for event in pygame.event.get():
            if event.type==pygame.QUIT:
                pygame.quit(); sys.exit()
            if event.type==pygame.KEYDOWN:
                if event.key==pygame.K_1:
                    level=1
                    waiting_level=False
                elif event.key==pygame.K_2:
                    level=2
                    waiting_level=False
                elif event.key==pygame.K_3:
                    level=3
                    waiting_level=False

    reset_game()
    if level==3:
        spawn_obstacles()

    running=True
    while running:
        for event in pygame.event.get():
            if event.type==pygame.QUIT:
                pygame.quit(); sys.exit()
            if event.type==pygame.KEYDOWN:
                if event.key==pygame.K_ESCAPE:
                    pygame.quit(); sys.exit()
                elif event.key==pygame.K_r and pygame.key.get_mods() & pygame.KMOD_CTRL:
                    game_loop()  # restart from beginning
                elif event.key==pygame.K_UP and direction!=(0,1):
                    direction=(0,-1)
                elif event.key==pygame.K_DOWN and direction!=(0,-1):
                    direction=(0,1)
                elif event.key==pygame.K_LEFT and direction!=(1,0):
                    direction=(-1,0)
                elif event.key==pygame.K_RIGHT and direction!=(-1,0):
                    direction=(1,0)
        if not game_over:
            move_snake()
            move_obstacles()
            screen.blit(game_bg,(0,0))
            draw_snake()
            draw_food()
            if level==3:
                draw_obstacles()
            text = font.render(f"Score: {score}", True, WHITE)
            screen.blit(text, (10,10))
        else:
            screen.fill(BLACK)
            text = font.render(f"Game Over! Score: {score}", True, WHITE)
            screen.blit(text, (WINDOW_WIDTH//2-100, WINDOW_HEIGHT//2))
        pygame.display.flip()
        clock.tick(snake_speed)

# ---------------- START ----------------
game_loop()


```
