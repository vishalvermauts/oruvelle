import Alexa from 'alexa-remote2';
import fs from 'fs';
const a = new Alexa();
a.init({ proxyOnly: true, proxyOwnIp: 'localhost', proxyPort: 3456, acceptLanguage: 'en-US', amazonPageProxyLanguage: 'en_US', amazonPage: process.env.AMAZON_PAGE || 'amazon.com', logger: console.log },
  e => { if (e) return console.log('ERR', e.message); fs.writeFileSync(import.meta.dirname + '/auth.json', JSON.stringify(a.cookieData)); console.log('LOGGED IN, saved auth.json'); process.exit(0); });
