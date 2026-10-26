// Exports the logged-in session for the Rust binary: node session.js
// Re-run it when Rust reports the session has expired (roughly every two weeks).
import fs from 'fs';
import { client, call, lights } from './alexa.js';

const a = await client();
const groups = await call(a, 'getSmarthomeGroups');
const routines = await call(a, 'getAutomationRoutines', 2000);

const devices = {};
for (const d of Object.values(a.serialNumbers)) {
  if (d.deviceFamily === 'VOX') continue;
  devices[d.accountName] = {
    serialNumber: d.serialNumber,
    deviceType: d.deviceType,
    deviceOwnerCustomerId: d.deviceOwnerCustomerId,
  };
}

const session = {
  cookie: a.cookie,
  csrf: a.csrf,
  host: `alexa.${a._options.amazonPage}`, // region-specific, e.g. alexa.amazon.com.au
  devices,
  lights: (await lights()).map(d => ({ applianceId: d.applianceId, name: d.friendlyName })),
  groups: Object.fromEntries(groups.applianceGroups.map(g => [g.name, g.applianceIds])),
  routines: routines.filter(r => r.name).map(r => ({ name: r.name, automationId: r.automationId, sequence: r.sequence })),
};

fs.writeFileSync(import.meta.dirname + '/session.json', JSON.stringify(session));
console.log(`session.json written: ${Object.keys(devices).length} devices, ${session.lights.length} lights, ${session.routines.length} routines`);
process.exit(0);
