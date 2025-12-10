A repository based on Febin John James's blog "The Impatient Programmer's Guide to Bevy and Rust" at https://aibodh.com/, with my own implementations to create a functioning video game prototype. 

Character spritesheets are provided from https://liberatedpixelcup.github.io/Universal-LPC-Spritesheet-Character-Generator/

Start the program by using 'cargo run'

Controls:

  WASD/Arrow Keys to move up, left, down, right.
  
  Shift to sprint.
  
  Space to jump.
  
  Enter/Space to advance in combat.
  
  1-6 to switch character models, each with different health and speed stats.

Gameplay: 

  Choose 1 out of 6 characters to play as.
  
    Male: 100 hp, 8 dmg, 140 speed
    
    Female: 95 hp, 7 dmg, 150 speed
    
    Crimson Count: 120 hp, 12 dmg, 180 speed
    
    Graveyard Reaper: 150 hp, 14 dmg, 120 speed
    
    Lantern Warden: 140 hp, 10 dmg, 110 speed
    
    Starlit Oracle: 85 hp, 9 dmg, 170 speed


  3 random enemies are spawned at the start of the game at random locations.
  
  To initiate combat, walk into an enemy and begin pressing Enter/Space to advance.
  
  Red sprites are spawned randomly across the map that can heal the player after combat.
  
  The game ends after all enemies are defeated or the player is defeated. You may choose to restart or quit by pressing R or Q respectively.
  

Known Errors: 

  Collision detection is not perfect, you may get stuck between certain tiles, and enemies can sometimes spawn on water tiles. Press R to restart in this case.
  

Miscellaneous: 
  
  The program features procedural generation, however the map is currently fixed to use the seed '12345'. If you wish to generate a new map, change the seed in /map/generate.rs using RngMode::Seeded(u64) or RngMode::RandomSeed for a random map every time the game is run.
  
  If you're being beaten in a fight, you can cheat and switch your character which starts you at max health again!
  
  If the game doesn't start and shows a graphics error, try updating your graphics drivers.
  
  Run 'cargo clean' to avoid storage bloat

  First time compilation will take a couple of minutes, please be patient :)
