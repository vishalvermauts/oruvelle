import Alexa from 'alexa-remote2';
import fs from 'fs';

export const FILE = import.meta.dirname + '/auth.json';
export const DEFAULT_DEVICE = process.env.ALEXA_DEVICE || "Vishal's Echo Show 5 2nd Gen";

let ready;
// One shared, logged-in client. First run: sign in at http://localhost:3456; the session is saved to auth.json.
export const client = () => ready ??= new Promise((ok, no) => {
  const a = new Alexa();
  a.init({
    cookie: fs.existsSync(FILE) ? JSON.parse(fs.readFileSync(FILE)) : undefined,
    proxyOnly: true, proxyOwnIp: 'localhost', proxyPort: 3456, amazonPage: process.env.AMAZON_PAGE || 'amazon.com',
    acceptLanguage: 'en-US', amazonPageProxyLanguage: 'en_US', logger: () => {},
  }, e => e ? no(e) : (fs.writeFileSync(FILE, JSON.stringify(a.cookieData)), ok(a)));
}).catch(e => { ready = null; throw e; });

// Promisify a callback-style library method: call(a, 'name', ...args)
export const call = (a, name, ...args) => new Promise((ok, no) => a[name](...args, (e, r) => e ? no(e) : ok(r)));
export const wait = ms => new Promise(r => setTimeout(r, ms));

// Smart-home devices are nested in the response; collect every node with an applianceId and friendlyName.
const flatten = (o, out = {}) => {
  if (o && typeof o === 'object') {
    if (o.applianceId && o.friendlyName) out[o.applianceId] = o;
    Object.values(o).forEach(v => flatten(v, out));
  }
  return out;
};

export async function devices() {
  const a = await client();
  return Object.values(a.serialNumbers).filter(d => d.deviceFamily !== 'VOX').map(d => d.accountName);
}

// All lights, or only those matching a light name or a room group name.
export async function lights(name) {
  const a = await client();
  const [groups, smart] = await Promise.all([call(a, 'getSmarthomeGroups'), call(a, 'getSmarthomeDevices')]);
  const all = Object.values(flatten(smart)).filter(d => d.applianceTypes?.includes('LIGHT'));
  if (!name) return all;
  const group = groups.applianceGroups.find(g => g.name.toLowerCase() === name.toLowerCase());
  return all.filter(d => d.friendlyName.toLowerCase() === name.toLowerCase() || group?.applianceIds.includes(d.applianceId));
}

export async function light(target, parameters) {
  const found = await lights(target);
  if (!found.length) throw new Error(`No lights match "${target}". Try alexa_lights_list.`);
  const a = await client();
  return call(a, 'executeSmarthomeDeviceAction', found.map(d => d.applianceId), parameters, 'APPLIANCE');
}

export async function blink(target, colorName, times = 3) {
  for (let i = 0; i < times; i++) {
    await light(target, { action: 'setColor', colorName });
    await wait(500);
    await light(target, { action: 'turnOff' });
    await wait(500);
  }
  return light(target, { action: 'setColor', colorName });
}

export async function say(device, mode, text) {
  const a = await client();
  return call(a, 'sendSequenceCommand', device || DEFAULT_DEVICE, mode, text);
}

export async function routine(name, device) {
  const a = await client();
  const all = await call(a, 'getAutomationRoutines', 2000);
  const hit = all.find(r => r.name?.toLowerCase() === name.toLowerCase()) || all.find(r => r.name?.toLowerCase().includes(name.toLowerCase()));
  if (!hit) throw new Error(`No routine matches "${name}". Try alexa_routines_list.`);
  return call(a, 'executeAutomationRoutine', device || DEFAULT_DEVICE, hit);
}

export async function routines() {
  const a = await client();
  return (await call(a, 'getAutomationRoutines', 2000)).map(r => r.name).filter(Boolean);
}
