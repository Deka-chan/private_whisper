import XCTest
@testable import PrivateWhisperCore

final class AudioMathTests: XCTestCase {
    func testRMSOfSilenceIsZero() {
        XCTAssertEqual(AudioMath.rms([0, 0, 0, 0]), 0, accuracy: 1e-6)
    }

    func testRMSOfConstant() {
        XCTAssertEqual(AudioMath.rms([0.5, 0.5, 0.5, 0.5]), 0.5, accuracy: 1e-6)
    }

    func testRMSOfEmptyIsZero() {
        XCTAssertEqual(AudioMath.rms([]), 0, accuracy: 1e-6)
    }

    func testDownmixStereoToMono() {
        // interleaved L,R,L,R -> среднее
        let stereo: [Float] = [1.0, 0.0, 0.0, 1.0]
        XCTAssertEqual(AudioMath.downmixToMono(interleaved: stereo, channels: 2), [0.5, 0.5])
    }

    func testDownmixMonoIsIdentity() {
        let mono: [Float] = [0.1, 0.2, 0.3]
        XCTAssertEqual(AudioMath.downmixToMono(interleaved: mono, channels: 1), mono)
    }

    func testTrimmedToVoiceRemovesSilentEnds() {
        let sr = 16000
        let silence = [Float](repeating: 0, count: sr / 2)        // 0.5с тишины
        let voice = [Float](repeating: 0.2, count: sr / 2)         // 0.5с «речи»
        let signal = silence + voice + silence                     // 1.5с всего
        let trimmed = AudioMath.trimmedToVoice(signal, sampleRate: sr)
        XCTAssertLessThan(trimmed.count, signal.count)             // края обрезаны
        XCTAssertGreaterThan(trimmed.count, sr / 2)                // речь сохранена
    }

    func testTrimmedToVoiceAllSilenceIsEmpty() {
        let trimmed = AudioMath.trimmedToVoice([Float](repeating: 0, count: 16000))
        XCTAssertTrue(trimmed.isEmpty)
    }
}
