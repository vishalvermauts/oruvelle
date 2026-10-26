import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';
import * as A from './alexa.js';

const s = new McpServer({ name: 'alexa', version: '2' });
const ok = r => ({ content: [{ type: 'text', text: JSON.stringify(r ?? 'ok') }] });
const device = z.string().optional().describe('Echo name. Defaults to the main Echo.');

s.tool('alexa_devices', 'List Echo / Alexa devices on the account.', {}, async () => ok(await A.devices()));

s.tool('alexa_speak', 'Make an Echo say text out loud.', { device, text: z.string() },
  async ({ device, text }) => ok(await A.say(device, 'speak', text)));

s.tool('alexa_command', 'Send any command to Alexa as if spoken: play music or Spotify, timers, reminders, shopping lists, questions, anything Alexa understands.',
  { device, text: z.string() }, async ({ device, text }) => ok(await A.say(device, 'textCommand', text)));

s.tool('alexa_volume', 'Set an Echo speaker volume, 0-100.', { device, level: z.number().int().min(0).max(100) },
  async ({ device, level }) => ok(await A.say(device, 'volume', level)));

s.tool('alexa_lights_list', 'List smart lights by name.', {}, async () => ok((await A.lights()).map(d => d.friendlyName)));

s.tool('alexa_light', 'Control smart lights by light name or room group (e.g. "Bedroom"): turn on or off, set brightness 0-100, or set a colour.',
  { target: z.string(), action: z.enum(['turnOn', 'turnOff', 'setBrightness', 'setColor']),
    brightness: z.number().int().min(0).max(100).optional(),
    color: z.string().optional().describe('Alexa colour name, e.g. "red", "blue", "warm_white".') },
  async ({ target, action, brightness, color }) => {
    const p = action === 'setBrightness' ? { action, brightness } : action === 'setColor' ? { action, colorName: color } : { action };
    return ok(await A.light(target, p));
  });

s.tool('alexa_blink', 'Blink lights in a colour, then leave them on that colour.',
  { target: z.string(), color: z.string(), times: z.number().int().min(1).max(20).default(3) },
  async ({ target, color, times }) => ok(await A.blink(target, color, times)));

s.tool('alexa_routines_list', 'List Alexa routines by name.', {}, async () => ok(await A.routines()));

s.tool('alexa_routine', 'Run an Alexa routine by name (partial match is fine).', { name: z.string(), device },
  async ({ name, device }) => ok(await A.routine(name, device)));

s.tool('alexa_notify', 'Universal status signal for IDE events: set or blink lights in a colour, and speak a message on an Echo. Use it when a task finishes or fails.',
  { target: z.string().optional().describe('Light or room group, e.g. "Lamp 1" or "Bedroom".'),
    color: z.string().optional(), blink: z.boolean().default(false), message: z.string().optional(), device },
  async ({ target, color, blink, message, device }) => {
    const out = {};
    if (target && color) out.lights = blink ? await A.blink(target, color, 3) : await A.light(target, { action: 'setColor', colorName: color });
    if (message) out.speech = await A.say(device, 'speak', message);
    return ok(out);
  });

await s.connect(new StdioServerTransport());
