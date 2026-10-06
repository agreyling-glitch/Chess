import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const source = readFileSync(new URL('../web/device-support.js',import.meta.url),'utf8');
const {isMobileDevice} = await import(`data:text/javascript,${encodeURIComponent(source)}`);
test('blocks phones, tablets and desktop-mode iPads, but allows touch laptops',()=>{
  for (const userAgent of ['iPhone','Android','iPad','Kindle']) assert.equal(isMobileDevice({userAgent}),true);
  assert.equal(isMobileDevice({userAgent:'Macintosh',maxTouchPoints:5}),true);
  assert.equal(isMobileDevice({userAgent:'Windows NT',maxTouchPoints:10}),false);
  assert.equal(isMobileDevice({userAgent:'Macintosh',maxTouchPoints:0}),false);
  assert.equal(isMobileDevice({userAgent:'Linux',userAgentData:{mobile:true}}),true);
});
