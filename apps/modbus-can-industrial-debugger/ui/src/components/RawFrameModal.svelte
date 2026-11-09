<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invokeIpc } from '../lib/ipc';
  import type { BusProtocol, IndustrialBusFrame } from '../lib/types';

  export let show = false;
  const dispatch = createEventDispatcher();

  let protocol: BusProtocol = 'modbus_rtu';
  let rawHex = '01 03 00 00 00 02 C4 0B';
  let isRequest = true;
  let analyzing = false;
  let decodedResult: IndustrialBusFrame | null = null;
  let errorMsg = '';

  function close() {
    show = false;
    decodedResult = null;
    errorMsg = '';
    dispatch('close');
  }

  async function dissect() {
    if (!rawHex.trim()) return;
    analyzing = true;
    errorMsg = '';
    decodedResult = null;

    try {
      const res = await invokeIpc<IndustrialBusFrame>('decode_raw_frame', {
        protocol,
        raw_hex: rawHex.trim(),
        is_request: isRequest,
      });
      decodedResult = res;
      dispatch('decoded', res);
    } catch (err: any) {
      errorMsg = err.message || 'Frame dissection failed';
    } finally {
      analyzing = false;
    }
  }
</script>

{#if show}
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4" on:click={close}>
    <div class="w-full max-w-2xl bg-slate-900 border border-slate-700 rounded-xl shadow-2xl p-6" on:click|stopPropagation>
      <div class="flex items-center justify-between pb-4 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <span class="text-amber-400 font-mono text-base">🔍</span>
          <h2 class="text-base font-semibold text-slate-100">Air-Gapped Raw Fieldbus Dissector</h2>
        </div>
        <button class="text-slate-400 hover:text-slate-200" on:click={close}>✕</button>
      </div>

      <div class="py-4 space-y-4">
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="proto-sel" class="block text-xs font-mono text-slate-400 uppercase mb-1">Bus Protocol</label>
            <select
              id="proto-sel"
              bind:value={protocol}
              class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
            >
              <option value="modbus_rtu">Modbus RTU (Serial RS-485)</option>
              <option value="modbus_tcp">Modbus TCP (Industrial Ethernet)</option>
              <option value="can_20a">CAN 2.0A (11-bit Standard)</option>
              <option value="can_20b">CAN 2.0B (29-bit Extended J1939)</option>
              <option value="can_fd">CAN-FD (Flexible Data-rate 64B)</option>
            </select>
          </div>
          <div>
            <label for="direction-sel" class="block text-xs font-mono text-slate-400 uppercase mb-1">Direction</label>
            <select
              id="direction-sel"
              bind:value={isRequest}
              class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-slate-100 text-sm focus:border-amber-500 focus:outline-none"
            >
              <option value={true}>Master Request / TX</option>
              <option value={false}>Slave Response / RX</option>
            </select>
          </div>
        </div>

        <div>
          <label for="hex-input" class="block text-xs font-mono text-slate-400 uppercase mb-1">Raw Frame Bytes (Hexadecimal)</label>
          <textarea
            id="hex-input"
            rows="3"
            bind:value={rawHex}
            placeholder="e.g. 01 03 00 00 00 02 C4 0B"
            class="w-full px-3 py-2 bg-slate-950 border border-slate-700 rounded-lg text-amber-300 font-mono text-xs focus:border-amber-500 focus:outline-none"
          ></textarea>
        </div>

        {#if errorMsg}
          <div class="p-3 bg-red-950/40 border border-red-800/60 rounded-lg text-xs text-red-300">{errorMsg}</div>
        {/if}

        {#if decodedResult}
          <div class="p-4 bg-slate-950 border border-slate-800 rounded-lg space-y-3 font-mono text-xs">
            <div class="text-slate-400 font-semibold border-b border-slate-800 pb-1">Dissected Payload Analysis</div>
            {#if decodedResult.modbus}
              <div class="grid grid-cols-2 gap-2 text-slate-300">
                <div>Unit ID: <span class="text-amber-400">{decodedResult.modbus.unit_id}</span></div>
                <div>Function: <span class="text-emerald-400">{decodedResult.modbus.function_name} (0x{decodedResult.modbus.function_code.toString(16).padStart(2, '0')})</span></div>
                <div>CRC Status: 
                  <span class="{decodedResult.modbus.crc_valid ? 'text-emerald-400' : 'text-red-400'} font-bold">
                    {decodedResult.modbus.crc_valid ? 'VALID (CRC-16 OK)' : 'CORRUPT'}
                  </span>
                </div>
                <div>Expected CRC: <span class="text-slate-400">0x{decodedResult.modbus.expected_crc.toString(16).padStart(4, '0')}</span></div>
              </div>
              {#if decodedResult.modbus.register_values.length > 0}
                <div class="text-slate-300">
                  Register Values: <span class="text-blue-400">[{decodedResult.modbus.register_values.join(', ')}]</span>
                </div>
              {/if}
            {/if}

            {#if decodedResult.can}
              <div class="grid grid-cols-2 gap-2 text-slate-300">
                <div>CAN ID: <span class="text-amber-400">{decodedResult.can.can_id_hex}</span></div>
                <div>DLC / Length: <span class="text-slate-400">{decodedResult.can.dlc} bytes</span></div>
              </div>
              {#if Object.keys(decodedResult.can.decoded_signals).length > 0}
                <div class="mt-2 pt-2 border-t border-slate-800">
                  <div class="text-slate-400 mb-1">Decoded Signals:</div>
                  {#each Object.entries(decodedResult.can.decoded_signals) as [k, v]}
                    <div class="text-emerald-300 text-xs">• {k}: <span class="font-bold">{v}</span></div>
                  {/each}
                </div>
              {/if}
            {/if}

            {#if decodedResult.security_alert}
              <div class="p-3 bg-red-950/60 border border-red-700/60 rounded text-red-200">
                <div class="font-bold text-xs uppercase tracking-wider text-red-400">⚠️ {decodedResult.security_alert.title}</div>
                <div class="mt-1 text-xs">{decodedResult.security_alert.description}</div>
                <div class="mt-1 text-slate-400 text-xs">Rule: {decodedResult.security_alert.iec_62443_rule}</div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex justify-end gap-3 pt-3 border-t border-slate-800">
        <button class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-lg text-xs font-medium" on:click={close}>
          Close
        </button>
        <button
          class="px-4 py-2 bg-amber-600 hover:bg-amber-500 text-slate-950 font-semibold rounded-lg text-xs transition-colors disabled:opacity-50"
          on:click={dissect}
          disabled={analyzing || !rawHex.trim()}
        >
          {analyzing ? 'Dissecting...' : 'Dissect Frame'}
        </button>
      </div>
    </div>
  </div>
{/if}
