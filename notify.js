// CLI for IDE hooks: node notify.js "<message>" [colour] [light-or-room] [blink]
import * as A from './alexa.js';

const [message, color = 'green', target = 'Lamp 1', blink] = process.argv.slice(2);
try {
  if (blink) await A.blink(target, color, 3);
  else await A.light(target, { action: 'setColor', colorName: color });
  if (message) await A.say(undefined, 'speak', message);
  console.log('ok');
  process.exit(0);
} catch (e) {
  console.error(e.message);
  process.exit(1);
}
