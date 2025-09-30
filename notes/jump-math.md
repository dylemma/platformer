# Parameters

- `jump_speed` - an impulse applied to the player when they jump
- `gravity` - an acceleration applied to the player while they are in the air
- `run_speed` - the max running speed for the player
- `fps` - frames per second

# Time to Apex

How long does the player take to reach the apex of their jump?

Upon jumping, the player's vertical velocity is set to `jump_speed`. 
Each frame, the player's vertical velocity is decreased by `gravity`.
The player's vertical velocity will reach 0 after `jump_speed / gravity` frames.
This is the "apex" of the jump.
The player will take just as many frames to get back to the ground.

When `fps = 60`, setting `jump_speed / gravity = 15` results in a half-second jump,
where the apex is reached after 15 frames, and the ground is reached after 30 frames.

# Apex Height

How high does the player jump?

Excluding other factors, the apex height is the sum of the player's vertical velocity
for each frame of the jump from `t=0` to `t=apex_time` (`t=15`).

```
Σ[t=0..apex_time](jump_speed - gravity*t)
=

Σ[t=0..apex_time](jump_speed) - gravity * Σ[t=0..apex_time](t)
=

(jump_speed * (apex_time + 1)) - (gravity * (apex_time * (apex_time + 1) / 2))
```

If we assume `jump_speed / gravity = 15`, and thus `apex_time = 15`, then:
`jump_speec = gravity * 15`, `gravity = jump_speed / 15`
```
(jump_speed * 16) - (gravity * 120)
=

(jump_speed * 16) - (jump_speed * 8)
=

8 * jump_speed
```

In an example where `jump_speed = 75`, the sum of velocities for each frame is `8 * 75 = 600`.
However, since velocity is measured in "per second" values, each frame, the player is translated
by `velocity / fps`, so with `fps = 60`, the apex height is `600 / 60 = 10` units.

# Max Distance

How far does the player jump?

If the player jumps while running at max speed, the horizontal distance traveled
before landing is `run_speed * jump_time = run_speed * 2 * time_to_apex`.

In the examples above, the `time_to_apex` is 15 frames, so the jump lasts 30 frames,
which is half a second, so the jump distance is `run_speed / 2`.