Notes
=====

# Wall Climbing

There's a state management issue with the wall controls, where if the player comes in contact with a wall at its "ledge",
it will *always* be considered as being on a ledge, even if they slide further down.
This results in the player unintentionally being able to climb all the way back up the wall after sliding down.
The intended behavior is for the "climbing" state to be used to let the player "make it" onto a platform they can just barely reach.

At the back of my mind, I'm thinking of Celeste's wall climbing ability, and wondering if I should just mimic that.
The player could climb any wall, limited by a stamina meter.
But I would rather not outright copy the game that inspired me to make this...

# Ledge Grab

I currently don't have a good plan for how to implement the player's ability to climb over a ledge, onto a platform.
The wall sensor stuff has the ability to detect ledges and steps, but the current controller logic makes the player "detach"
from the wall once the sensor goes below "ledge", and results in a vertical jitter as the player "leaves" the grabbable portion of the wall
then immediately re-grabs as they fall for 1 frame.

The intended behavior is for the player to climb up *and onto* the platform when they hold towards the ledge they grabbed (or up).

I should reconsider whether I want a "climb animation", vs just letting the player clip through the corner of a platform.
A climb animation could be achievable by just taking full control of the character's position during the animation, locking the controls.
But I'm not sure that sounds "fun".
If I just let the character clip through corners, I think that's closer to Celeste's behavior.
It would be confusing to achieve with the KinematicCharacterController utility, though...

Maybe I could set up a smaller hitbox for the character, and do a shape-cast in the world when the player hits a platform.
If the smaller hitbox would not hit that same platform when given the same trajectory, just let the player move in that direction.
This idea seems exciting, but needs refinement and consideration of edge cases (pun intended).
This might also work for the "anti head-bonking" feature that Celeste has, where if you jump or dash upward into the corner of a platform,
you automatically get pushed to the side and get to keep your momentum.

# Dash

I want a dash power like in Celeste, where the player boosts a few units in the chosen direction.
That should be achievable via the Decaying Force struct I put together to represent wall jump forces.
There could also be a timer attached to the dash state, to spawn "afterimage" sprites along the trajectory of the dash.