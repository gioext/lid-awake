import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import App from './App.svelte';

const { invokeMock, listenMock, startDraggingMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
  listenMock: vi.fn().mockResolvedValue(() => undefined),
  startDraggingMock: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));
vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ startDragging: startDraggingMock }),
}));

describe('Lid Awake UI', () => {
  afterEach(() => cleanup());

  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockClear();
    startDraggingMock.mockClear();
  });

  it.each([
    ['enabled', 'OFF'],
    ['disabled', 'ON'],
  ] as const)('maps %s to %s', async (status, label) => {
    invokeMock.mockResolvedValueOnce(status);
    render(App);

    expect(await screen.findByText(label)).toBeInTheDocument();
    expect(screen.getByRole('button')).toHaveAttribute(
      'aria-pressed',
      status === 'disabled' ? 'true' : 'false',
    );
  });

  it('keeps the OFF toggle fixed and shows updating in the title bar', async () => {
    let resolveChange: (value: 'applied') => void = () => undefined;
    const changePromise = new Promise<'applied'>((resolve) => {
      resolveChange = resolve;
    });

    invokeMock
      .mockResolvedValueOnce('enabled')
      .mockReturnValueOnce(changePromise)
      .mockResolvedValueOnce('disabled');

    render(App);
    const button = await screen.findByRole('button', { name: 'Lid Awakeをオンにする' });
    await fireEvent.click(button);

    expect(screen.getByText('UPDATING…')).toBeInTheDocument();
    expect(screen.getByText('OFF')).toBeInTheDocument();
    expect(screen.queryByText('ON')).not.toBeInTheDocument();
    expect(button).toHaveAttribute('aria-pressed', 'false');
    expect(invokeMock).toHaveBeenNthCalledWith(2, 'set_sleep_disabled', { disabled: true });

    resolveChange('applied');
    expect(await screen.findByText('ON')).toBeInTheDocument();
  });

  it('maps an ON click to disabled=false', async () => {
    invokeMock
      .mockResolvedValueOnce('disabled')
      .mockResolvedValueOnce('applied')
      .mockResolvedValueOnce('enabled');

    render(App);
    const button = await screen.findByRole('button', { name: 'Lid Awakeをオフにする' });
    await fireEvent.click(button);

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith('set_sleep_disabled', { disabled: false });
    });
    expect(await screen.findByText('OFF')).toBeInTheDocument();
  });

  it('shows a retry action after the initial read fails', async () => {
    invokeMock.mockRejectedValueOnce(new Error('pmset failed'));
    render(App);

    expect(await screen.findByText('RETRY')).toBeInTheDocument();
    expect(screen.getByText('ERROR')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '状態取得を再試行' })).toHaveAttribute(
      'title',
      'Error: pmset failed',
    );
  });

  it('keeps the known state visible when an update fails', async () => {
    invokeMock
      .mockResolvedValueOnce('enabled')
      .mockRejectedValueOnce(new Error('authorization failed'))
      .mockResolvedValueOnce('enabled');

    render(App);
    const button = await screen.findByRole('button', { name: 'Lid Awakeをオンにする' });
    await fireEvent.click(button);

    expect(await screen.findByText('UPDATE FAILED')).toBeInTheDocument();
    expect(screen.getByText('OFF')).toBeInTheDocument();
    expect(button).toHaveAttribute('title', 'Error: authorization failed');
  });

  it('returns to the real state without an error after cancellation', async () => {
    invokeMock
      .mockResolvedValueOnce('enabled')
      .mockResolvedValueOnce('cancelled')
      .mockResolvedValueOnce('enabled');

    render(App);
    const button = await screen.findByRole('button', { name: 'Lid Awakeをオンにする' });
    await fireEvent.click(button);

    expect(await screen.findByText('OFF')).toBeInTheDocument();
    expect(screen.queryByText(/FAILED|ERROR/)).not.toBeInTheDocument();
  });
});
