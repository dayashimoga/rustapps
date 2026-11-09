<script lang="ts">
  import type { IndustrialBusFrame } from '../lib/types';

  export let frame: IndustrialBusFrame | null = null;
</script>

<div class="flex flex-col h-full bg-slate-950 overflow-y-auto p-4 space-y-4">
  {#if frame}
    <div class="flex items-center justify-between pb-3 border-b border-slate-800">
      <div>
        <div class="text-xs font-mono text-slate-500 uppercase tracking-wider">Dissection Inspector</div>
        <div class="text-base font-bold text-slate-100 flex items-center gap-2 mt-0.5">
          <span>Frame #{frame.frame_index}</span>
          <span class="text-xs px-2 py-0.5 rounded font-mono font-normal bg-slate-800 text-slate-300">
            {frame.protocol.toUpperCase()}
          </span>
          <span class="text-xs text-slate-400 font-mono">+{frame.timestamp_micros}µs</span>
        </div>
      </div>
      <div class="text-xs font-mono text-slate-400">
        Direction: <span class="text-amber-400 font-semibold">{frame.direction}</span>
      </div>
    </div>

    {#if frame.security_alert}
      <div class="p-4 bg-red-950/40 border border-red-800/80 rounded-xl space-y-2">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="text-red-400 font-bold text-sm">🚨 {frame.security_alert.title}</span>
            <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-red-900/60 text-red-200 border border-red-700/60 uppercase font-semibold">
              {frame.security_alert.severity}
            </span>
          </div>
          <span class="text-xs font-mono text-red-300/80">{frame.security_alert.iec_62443_rule}</span>
        </div>
        <p class="text-xs text-red-200 leading-relaxed font-sans">{frame.security_alert.description}</p>
        <div class="p-2.5 bg-black/40 rounded border border-red-900/40 text-xs text-amber-300 font-mono">
          <span class="text-slate-400 font-semibold">Mitigation Playbook:</span> {frame.security_alert.mitigation_guidance}
        </div>
      </div>
    {/if}

    {#if frame.modbus}
      <div class="p-4 bg-slate-900 border border-slate-800 rounded-xl space-y-3 font-mono text-xs">
        <div class="text-xs font-bold text-slate-300 uppercase tracking-wider border-b border-slate-800 pb-2">
          Modbus Protocol Data Unit (PDU)
        </div>

        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">Slave Unit ID</div>
            <div class="text-sm font-bold text-amber-400 mt-0.5">{frame.modbus.unit_id} (0x{frame.modbus.unit_id.toString(16).padStart(2, '0')})</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">Function Code</div>
            <div class="text-sm font-bold text-emerald-400 mt-0.5">FC {frame.modbus.function_code}</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">Function Name</div>
            <div class="text-xs font-bold text-slate-200 mt-0.5 truncate">{frame.modbus.function_name}</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">CRC-16 Integrity</div>
            <div class="text-xs font-bold mt-0.5 {frame.modbus.crc_valid ? 'text-emerald-400' : 'text-red-400'}">
              {frame.modbus.crc_valid ? 'VALID (PASS)' : 'CORRUPTED (FAIL)'}
            </div>
          </div>
        </div>

        {#if frame.modbus.register_address !== undefined}
          <div class="p-3 bg-slate-950 rounded border border-slate-800 flex items-center justify-between">
            <span class="text-slate-400">Target Register Address:</span>
            <span class="text-amber-300 font-bold">{frame.modbus.register_address} (0x{frame.modbus.register_address.toString(16).padStart(4, '0')})</span>
          </div>
        {/if}

        {#if frame.modbus.register_values && frame.modbus.register_values.length > 0}
          <div class="space-y-1.5 pt-2">
            <div class="text-xs text-slate-400 font-semibold">Decoded Register Table:</div>
            <div class="overflow-x-auto">
              <table class="w-full text-left text-xs border border-slate-800">
                <thead class="bg-slate-950 text-slate-500">
                  <tr>
                    <th class="p-2 border-b border-slate-800">Index</th>
                    <th class="p-2 border-b border-slate-800">Hex</th>
                    <th class="p-2 border-b border-slate-800">Decimal</th>
                    <th class="p-2 border-b border-slate-800">Binary Bitfield</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-800">
                  {#each frame.modbus.register_values as val, idx}
                    <tr class="hover:bg-slate-800/40">
                      <td class="p-2 text-slate-500">Reg {idx}</td>
                      <td class="p-2 text-amber-300">0x{val.toString(16).padStart(4, '0').toUpperCase()}</td>
                      <td class="p-2 text-blue-300">{val}</td>
                      <td class="p-2 text-emerald-400 font-mono text-[10px]">{val.toString(2).padStart(16, '0')}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          </div>
        {/if}
      </div>
    {/if}

    {#if frame.can}
      <div class="p-4 bg-slate-900 border border-slate-800 rounded-xl space-y-3 font-mono text-xs">
        <div class="text-xs font-bold text-slate-300 uppercase tracking-wider border-b border-slate-800 pb-2">
          CAN Frame Telemetry
        </div>

        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">CAN Identifier</div>
            <div class="text-sm font-bold text-amber-400 mt-0.5">{frame.can.can_id_hex}</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">Format</div>
            <div class="text-xs font-bold text-slate-200 mt-0.5">{frame.can.is_extended ? '29-bit Extended' : '11-bit Standard'}</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">Data Length (DLC)</div>
            <div class="text-sm font-bold text-blue-400 mt-0.5">{frame.can.dlc} bytes</div>
          </div>
          <div class="p-2.5 bg-slate-950 rounded border border-slate-800">
            <div class="text-[10px] text-slate-500 uppercase">CAN-FD Mode</div>
            <div class="text-xs font-bold mt-0.5 {frame.can.is_fd ? 'text-emerald-400' : 'text-slate-400'}">
              {frame.can.is_fd ? 'YES (64B payload)' : 'NO (Classical)'}
            </div>
          </div>
        </div>

        <div class="p-3 bg-slate-950 rounded border border-slate-800">
          <div class="text-[10px] text-slate-500 uppercase mb-1">Payload Hex Stream:</div>
          <div class="text-amber-300 font-bold tracking-widest">{frame.can.payload_hex}</div>
        </div>

        {#if Object.keys(frame.can.decoded_signals).length > 0}
          <div class="space-y-2 pt-2">
            <div class="text-xs text-slate-400 font-semibold">Decoded Fieldbus Signals (J1939 / OBD-II):</div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
              {#each Object.entries(frame.can.decoded_signals) as [signal, val]}
                <div class="p-3 bg-slate-950 rounded border border-slate-800/80 flex items-center justify-between">
                  <span class="text-slate-400">{signal}:</span>
                  <span class="text-emerald-300 font-bold">{val}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    {/if}

  {:else}
    <div class="flex-1 flex flex-col items-center justify-center text-center p-12 text-slate-500 font-mono text-xs">
      <span class="text-4xl mb-3 opacity-40">📡</span>
      <div>Select a fieldbus packet from the waterfall to inspect register values, decoded CAN signals, and IEC 62443 security rules.</div>
    </div>
  {/if}
</div>
