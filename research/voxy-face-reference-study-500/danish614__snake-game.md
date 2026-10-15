# Facial rendering source scan: danish614/snake-game

Repository: https://github.com/danish614/snake-game

Pinned source revision: b71b5a0678b5aa30dd56828857eb3fb981773b1f

Discovery topic: tongue animation

Description: This Python + Pygame project creates a realistic dark snake animation that smoothly follows the mouse cursor with wave-like body motion and a flicking tongue effect.

Reported license: unknown; inspect license files before adopting code/assets

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: []

Topic paths: {}

## Source file: pythonsnake.py

{
  "path": "pythonsnake.py",
  "blob_sha": "7b8c9fab176475490d6f9475f3f38d13522d05b7",
  "sha256": "6c0886e4bace19d3832c16fe3e80cde1edc59956b01533ad4e3a08af0491c4ce",
  "lines": 142,
  "bytes": 4592,
  "selection_score": 0,
  "keyword_line_counts": {
    "eye": 3,
    "mouth": 2,
    "tongue": 16,
    "face": 1,
    "lip": 3
  },
  "url": "https://github.com/danish614/snake-game/blob/b71b5a0678b5aa30dd56828857eb3fb981773b1f/pythonsnake.py",
  "function_signature_candidates": [
    {
      "line": 28,
      "text": "def lerp(a, b, t): return a + (b - a) * t"
    }
  ]
}

```text

import pygame, math, sys

pygame.init()
W, H = 1000, 700
screen = pygame.display.set_mode((W, H))
clock = pygame.time.Clock()
pygame.display.set_caption("Dark Snake with Flicking Tongue")

NUM_SEGMENTS = 60
SEG_LEN = 14
HEAD_RADIUS = 22
BODY_COLOR = (25, 25, 25)
HIGHLIGHT = (70, 70, 70)
EYE_COLOR = (220, 30, 30)
TONGUE_COLOR = (255, 50, 50)
WAVE_FREQ = 5.0
WAVE_AMP = 7.0
TONGUE_FREQ = 15.0
TONGUE_LEN = 18
SPEED_BASE = 220
SMOOTH_T = 0.55


center = (W//2, H//2)
snake = [(center[0], center[1] + i * SEG_LEN) for i in range(NUM_SEGMENTS)]
time_acc = 0.0

def lerp(a, b, t): return a + (b - a) * t

running = True
while running:
    dt = clock.tick(60) / 1000.0
    time_acc += dt

    for e in pygame.event.get():
        if e.type == pygame.QUIT: running = False
        if e.type == pygame.KEYDOWN and e.key == pygame.K_ESCAPE: running = False

    mx, my = pygame.mouse.get_pos()

    
    hx, hy = snake[0]
    dx, dy = mx - hx, my - hy
    distance = math.hypot(dx, dy)
    angle = math.atan2(dy, dx) if distance else 0
    step = SPEED_BASE * dt * (0.5 + min(distance/300,1.5))

    
    perp_ang = angle + math.pi/2
    wave = math.sin(time_acc * WAVE_FREQ) * WAVE_AMP
    target_x = mx + math.cos(perp_ang) * wave
    target_y = my + math.sin(perp_ang) * wave

    move_dx, move_dy = target_x - hx, target_y - hy
    move_d = math.hypot(move_dx, move_dy)
    if move_d > step:
        hx += (move_dx / move_d) * step
        hy += (move_dy / move_d) * step
    snake[0] = (hx, hy)

    
    for i in range(1, NUM_SEGMENTS):
        px, py = snake[i-1]
        sx, sy = snake[i]
        dx, dy = px - sx, py - sy
        d = math.hypot(dx, dy)
        if d == 0: continue
        target_x = px - (dx/d) * SEG_LEN
        target_y = py - (dy/d) * SEG_LEN
        phase = time_acc * WAVE_FREQ - i * 0.25
        lateral = math.sin(phase) * WAVE_AMP * (1 - i/NUM_SEGMENTS)
        ang = math.atan2(dy, dx) + math.pi / 2
        target_x += math.cos(ang) * lateral
        target_y += math.sin(ang) * lateral
        snake[i] = (lerp(sx, target_x, SMOOTH_T), lerp(sy, target_y, SMOOTH_T))

     
    screen.fill((12,12,12))

    
    for i in range(NUM_SEGMENTS-1, 0, -1):
        x, y = snake[i]
        t = i / NUM_SEGMENTS
        r = int(HEAD_RADIUS * (0.3 + 0.7*(1 - t)))
        shade = int(lerp(20, 70, 1 - t))
        pygame.draw.circle(screen, (shade, shade, shade), (int(x), int(y)), r)

    
    hx, hy = snake[0]
    nx, ny = snake[1]
    head_ang = math.atan2(ny - hy, nx - hx) + math.pi

    head_surf = pygame.Surface((HEAD_RADIUS*3, HEAD_RADIUS*2), pygame.SRCALPHA)
    pygame.draw.polygon(
        head_surf, BODY_COLOR,
        [(0, HEAD_RADIUS), (HEAD_RADIUS*3, HEAD_RADIUS*0.25), (HEAD_RADIUS*3, HEAD_RADIUS*1.75)]
    )
    pygame.draw.line(head_surf, HIGHLIGHT, (HEAD_RADIUS, HEAD_RADIUS*0.3), (HEAD_RADIUS*2.5, HEAD_RADIUS*0.3), 2)

    
    ex = HEAD_RADIUS*2.1
    ey1, ey2 = HEAD_RADIUS*0.65, HEAD_RADIUS*1.35
    pygame.draw.ellipse(head_surf, EYE_COLOR, (ex, ey1, 5, 8))
    pygame.draw.ellipse(head_surf, EYE_COLOR, (ex, ey2, 5, 8))

    
    mouth_open = min(1.0, distance / 200)
    my_offset = int(mouth_open * 3)
    pygame.draw.line(head_surf, (180,20,20),
                     (HEAD_RADIUS*0.8, HEAD_RADIUS + my_offset),
                     (HEAD_RADIUS*2.5, HEAD_RADIUS + my_offset), 2)

    
    tongue_phase = math.sin(time_acc * TONGUE_FREQ) * 0.5 + 0.5  
    tongue_len_dynamic = TONGUE_LEN * (0.5 + tongue_phase*0.5)
    tongue_angle = head_ang
    tongue_end_x = hx + math.cos(tongue_angle) * tongue_len_dynamic
    tongue_end_y = hy + math.sin(tongue_angle) * tongue_len_dynamic
    pygame.draw.line(screen, TONGUE_COLOR, (hx, hy), (tongue_end_x, tongue_end_y), 2)

    fork_offset = 3
    pygame.draw.line(screen, TONGUE_COLOR, (tongue_end_x, tongue_end_y),
                     (tongue_end_x - math.sin(tongue_angle)*fork_offset,
                      tongue_end_y + math.cos(tongue_angle)*fork_offset), 1)
    pygame.draw.line(screen, TONGUE_COLOR, (tongue_end_x, tongue_end_y),
                     (tongue_end_x + math.sin(tongue_angle)*fork_offset,
                      tongue_end_y - math.cos(tongue_angle)*fork_offset), 1)

    
    rot_head = pygame.transform.rotozoom(head_surf, -math.degrees(head_ang), 1.0)
    rect_head = rot_head.get_rect(center=(int(hx), int(hy)))
    screen.blit(rot_head, rect_head.topleft)

    
    for i in range(0, NUM_SEGMENTS, 4):
        x, y = snake[i]
        pygame.draw.circle(screen, (30,30,30,60), (int(x), int(y)), int(HEAD_RADIUS*0.6), 0)

    pygame.display.flip()

pygame.quit()
sys.exit()


```
