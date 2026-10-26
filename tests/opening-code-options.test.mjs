import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
const source=readFileSync(new URL('../web/game-storage.js',import.meta.url),'utf8');
const context=vm.createContext({});
vm.runInContext(source.slice(source.indexOf('function savedOpeningOptions'),source.indexOf('function readWindowState')),context);
const savedOpeningOptions=context.savedOpeningOptions;
test('each ECO code has one all-games option alongside exact variations',()=>{
 const choices=[{eco:'A85',opening:'Dutch Defense'},{eco:'A85',opening:'Dutch Defense: Variation'},{eco:'B20',opening:'Sicilian Defense'},{eco:'',opening:'Custom'}];
 const options=savedOpeningOptions(choices);
 assert.equal(options.filter(o=>o.text==='All A85 games').length,1);
 assert.ok(options.some(o=>o.text==='All B20 games'&&o.value==='["B20","*"]'));
 assert.ok(options.some(o=>o.value==='["A85","Dutch Defense: Variation"]'));
 assert.equal(options.length,6);
});
