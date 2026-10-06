export function isMobileDevice(device = navigator) {
  return device.userAgentData?.mobile === true
    || /Android|iPhone|iPad|iPod|Mobile|Tablet|Silk|Kindle/i.test(device.userAgent || '')
    || (/Macintosh/i.test(device.userAgent || '') && device.maxTouchPoints > 1);
}
