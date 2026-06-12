final class UniFFIException extends \RuntimeException
{
    public function __construct(public mixed $error)
    {
        parent::__construct(UniFFIRuntime::describeError($error));
    }
}

final class UniFFIRuntime
{
    private const CDEF = <<<'UNIFFI_CDEF'
{{ cdef }}
UNIFFI_CDEF;

    private const LIBRARY = '{{ library }}';
    private const CALL_SUCCESS = 0;
    private const CALL_ERROR = 1;
    private const CALL_UNEXPECTED_ERROR = 2;
    private const CALL_CANCELLED = 3;

    private static ?\FFI $ffi = null;
    private static bool $initialized = false;
    private static bool $initializing = false;
    private static int $nextCallbackHandle = 1;
    private static array $callbackHandles = [];
    private static array $keepAlive = [];

    public static function ffi(): \FFI
    {
        if (self::$ffi === null) {
            self::$ffi = \FFI::cdef(self::CDEF, self::LIBRARY);
        }
        return self::$ffi;
    }

    public static function ensureInitialized(): void
    {
        if (self::$initialized || self::$initializing) {
            return;
        }

        self::$initializing = true;
        try {
            $ffi = self::ffi();
            $contractVersion = $ffi->{'{{ contract_version_fn }}'}();
            if ($contractVersion !== {{ contract_version }}) {
                throw new \RuntimeException('UniFFI contract version mismatch: clean and rebuild the Rust component and PHP bindings');
            }

            {%- if !omit_checksums %}
            {%- for item in checksums %}
            if ($ffi->{'{{ item.fn_name }}'}() !== {{ item.checksum }}) {
                throw new \RuntimeException('UniFFI API checksum mismatch for {{ item.fn_name }}');
            }
            {%- endfor %}
            {%- endif %}

            {%- for initializer in callback_initializers %}
            {{ initializer }}
            {%- endfor %}

            self::$initialized = true;
        } finally {
            self::$initializing = false;
        }
    }

    public static function rustCall(string $fn, mixed ...$args): mixed
    {
        return self::rustCallWithError($fn, null, ...$args);
    }

    public static function rustCallWithError(string $fn, mixed $errorSpec, mixed ...$args): mixed
    {
        self::ensureInitialized();

        $ffi = self::ffi();
        $status = $ffi->new('RustCallStatus');
        $args[] = \FFI::addr($status);
        $result = $ffi->{$fn}(...$args);

        self::checkCallStatus($status, $errorSpec);
        return $result;
    }

    private static function checkCallStatus(\FFI\CData $status, mixed $errorSpec): void
    {
        switch ($status->code) {
            case self::CALL_SUCCESS:
                return;
            case self::CALL_ERROR:
                if ($errorSpec !== null) {
                    throw new UniFFIException(self::liftSerialized($status->errorBuf, $errorSpec));
                }
                $message = self::consumeErrorBuffer($status->errorBuf, 'Rust call returned an error');
                throw new \RuntimeException($message);
            case self::CALL_UNEXPECTED_ERROR:
                $message = self::consumeErrorBuffer($status->errorBuf, 'Rust panic');
                throw new \RuntimeException($message);
            case self::CALL_CANCELLED:
                throw new \RuntimeException('Rust future was cancelled');
            default:
                throw new \RuntimeException('Unknown RustCallStatus code: ' . $status->code);
        }
    }

    public static function describeError(mixed $error): string
    {
        if (is_object($error) && property_exists($error, 'variant')) {
            $message = get_class($error) . '::' . $error->variant;
            if (property_exists($error, 'fields') && $error->fields !== []) {
                $message .= ' ' . json_encode($error->fields, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
            }
            return $message;
        }
        if (is_object($error)) {
            return get_class($error);
        }
        if (is_scalar($error) || $error === null) {
            return (string) $error;
        }
        return 'UniFFI error';
    }

    private static function consumeErrorBuffer(\FFI\CData $buf, string $fallback): string
    {
        if ((int) $buf->len === 0) {
            self::freeRustBuffer($buf);
            return $fallback;
        }
        return self::liftString($buf);
    }

    public static function lowerBool(bool $value): int
    {
        return $value ? 1 : 0;
    }

    public static function liftBool(int $value): bool
    {
        return $value !== 0;
    }

    public static function lowerInt8(int $value): int
    {
        return self::intInRange($value, -128, 127, 'i8');
    }

    public static function liftInt8(int $value): int
    {
        return $value;
    }

    public static function lowerUInt8(int $value): int
    {
        return self::intInRange($value, 0, 255, 'u8');
    }

    public static function liftUInt8(int $value): int
    {
        return $value;
    }

    public static function lowerInt16(int $value): int
    {
        return self::intInRange($value, -32768, 32767, 'i16');
    }

    public static function liftInt16(int $value): int
    {
        return $value;
    }

    public static function lowerUInt16(int $value): int
    {
        return self::intInRange($value, 0, 65535, 'u16');
    }

    public static function liftUInt16(int $value): int
    {
        return $value;
    }

    public static function lowerInt32(int $value): int
    {
        return self::intInRange($value, -2147483648, 2147483647, 'i32');
    }

    public static function liftInt32(int $value): int
    {
        return $value;
    }

    public static function lowerUInt32(int $value): int
    {
        return self::intInRange($value, 0, 4294967295, 'u32');
    }

    public static function liftUInt32(int $value): int
    {
        return $value;
    }

    public static function lowerInt64(int $value): int
    {
        return $value;
    }

    public static function liftInt64(int $value): int
    {
        return $value;
    }

    public static function lowerUInt64(int $value): int
    {
        if ($value < 0) {
            throw new \RangeException('u64 requires a non-negative integer');
        }
        return $value;
    }

    public static function liftUInt64(int $value): int
    {
        return $value;
    }

    public static function lowerFloat32(float $value): float
    {
        return $value;
    }

    public static function liftFloat32(float $value): float
    {
        return $value;
    }

    public static function lowerFloat64(float $value): float
    {
        return $value;
    }

    public static function liftFloat64(float $value): float
    {
        return $value;
    }

    public static function lowerString(string $value): \FFI\CData
    {
        if (!preg_match('//u', $value)) {
            throw new \InvalidArgumentException('UniFFI string values must be valid UTF-8');
        }
        return self::lowerRawBytes($value);
    }

    public static function liftString(\FFI\CData $buf): string
    {
        return self::liftRawBytes($buf);
    }

    public static function lowerBytes(string $value): \FFI\CData
    {
        $writer = [];
        self::writeByteString($writer, $value);
        return self::lowerRawBytes(self::bytesFromArray($writer));
    }

    public static function liftBytes(\FFI\CData $buf): string
    {
        $data = self::liftRawBytes($buf);
        $reader = ['data' => $data, 'offset' => 0];
        $value = self::readByteString($reader);
        if ($reader['offset'] !== strlen($data)) {
            throw new \RuntimeException('UniFFI bytes RustBuffer contained trailing bytes');
        }
        return $value;
    }

    private static function lowerRawBytes(string $value): \FFI\CData
    {
        $ffi = self::ffi();
        $len = strlen($value);
        $foreignBytes = $ffi->new('ForeignBytes');
        $foreignBytes->len = $len;

        if ($len > 0) {
            $bytes = $ffi->new("uint8_t[$len]", false);
            \FFI::memcpy($bytes, $value, $len);
            $foreignBytes->data = \FFI::cast('const uint8_t *', $bytes);
        } else {
            $foreignBytes->data = null;
        }

        return self::rustCall('{{ rustbuffer_from_bytes_fn }}', $foreignBytes);
    }

    private static function liftRawBytes(\FFI\CData $buf): string
    {
        try {
            if ((int) $buf->len === 0) {
                return '';
            }
            return \FFI::string($buf->data, (int) $buf->len);
        } finally {
            self::freeRustBuffer($buf);
        }
    }

    public static function lowerSerialized(mixed $value, mixed $spec): \FFI\CData
    {
        $writer = [];
        self::writeValue($writer, $value, $spec);
        return self::lowerRawBytes(self::bytesFromArray($writer));
    }

    public static function liftSerialized(\FFI\CData $buf, mixed $spec): mixed
    {
        $data = self::liftRawBytes($buf);
        $reader = ['data' => $data, 'offset' => 0];
        $value = self::readValue($reader, $spec);
        if ($reader['offset'] !== strlen($data)) {
            throw new \RuntimeException('UniFFI RustBuffer contained trailing bytes');
        }
        return $value;
    }

    public static function readValue(array &$reader, mixed $spec): mixed
    {
        if (is_string($spec)) {
            return match ($spec) {
                'u8' => self::readUInt8($reader),
                'i8' => self::readInt8($reader),
                'u16' => self::readUInt16($reader),
                'i16' => self::readInt16($reader),
                'u32' => self::readUInt32($reader),
                'i32' => self::readInt32($reader),
                'u64' => self::readUInt64($reader),
                'i64' => self::readInt64($reader),
                'f32' => self::readFloat32($reader),
                'f64' => self::readFloat64($reader),
                'bool' => self::readBool($reader),
                'string' => self::readString($reader),
                'bytes' => self::readByteString($reader),
                'timestamp' => self::readTimestamp($reader),
                'duration' => self::readDuration($reader),
                default => throw new \InvalidArgumentException('Unknown UniFFI type spec: ' . $spec),
            };
        }

        $kind = $spec[0] ?? null;
        return match ($kind) {
            'record', 'enum', 'object', 'callback-object', 'callback' => self::classForSpec($spec[1])::uniffiRead($reader),
            'optional' => self::readOptional($reader, $spec[1]),
            'sequence' => self::readSequence($reader, $spec[1]),
            'map' => self::readMap($reader, $spec[1], $spec[2]),
            default => throw new \InvalidArgumentException('Unknown UniFFI compound type spec'),
        };
    }

    public static function writeValue(array &$writer, mixed $value, mixed $spec): void
    {
        if (is_string($spec)) {
            match ($spec) {
                'u8' => self::writeUInt8($writer, $value),
                'i8' => self::writeInt8($writer, $value),
                'u16' => self::writeUInt16($writer, $value),
                'i16' => self::writeInt16($writer, $value),
                'u32' => self::writeUInt32($writer, $value),
                'i32' => self::writeInt32($writer, $value),
                'u64' => self::writeUInt64($writer, $value),
                'i64' => self::writeInt64($writer, $value),
                'f32' => self::writeFloat32($writer, $value),
                'f64' => self::writeFloat64($writer, $value),
                'bool' => self::writeBool($writer, $value),
                'string' => self::writeString($writer, $value),
                'bytes' => self::writeByteString($writer, $value),
                'timestamp' => self::writeTimestamp($writer, $value),
                'duration' => self::writeDuration($writer, $value),
                default => throw new \InvalidArgumentException('Unknown UniFFI type spec: ' . $spec),
            };
            return;
        }

        $kind = $spec[0] ?? null;
        match ($kind) {
            'record', 'enum', 'object', 'callback-object', 'callback' => self::classForSpec($spec[1])::uniffiWrite($value, $writer),
            'optional' => self::writeOptional($writer, $value, $spec[1]),
            'sequence' => self::writeSequence($writer, $value, $spec[1]),
            'map' => self::writeMap($writer, $value, $spec[1], $spec[2]),
            default => throw new \InvalidArgumentException('Unknown UniFFI compound type spec'),
        };
    }

    public static function readBool(array &$reader): bool
    {
        return self::readInt8($reader) !== 0;
    }

    public static function writeBool(array &$writer, bool $value): void
    {
        self::writeInt8($writer, $value ? 1 : 0);
    }

    public static function readInt8(array &$reader): int
    {
        $value = self::readUInt8($reader);
        return $value >= 0x80 ? $value - 0x100 : $value;
    }

    public static function writeInt8(array &$writer, int $value): void
    {
        self::writeUInt8($writer, $value & 0xff);
    }

    public static function readUInt8(array &$reader): int
    {
        return ord(self::readRawBytes($reader, 1));
    }

    public static function writeUInt8(array &$writer, int $value): void
    {
        $writer[] = self::intInRange($value, 0, 255, 'u8');
    }

    public static function readInt16(array &$reader): int
    {
        $value = self::readUInt16($reader);
        return $value >= 0x8000 ? $value - 0x10000 : $value;
    }

    public static function writeInt16(array &$writer, int $value): void
    {
        self::writeUInt16($writer, $value < 0 ? $value + 0x10000 : $value);
    }

    public static function readUInt16(array &$reader): int
    {
        return unpack('nvalue', self::readRawBytes($reader, 2))['value'];
    }

    public static function writeUInt16(array &$writer, int $value): void
    {
        self::appendPacked($writer, pack('n', self::intInRange($value, 0, 0xffff, 'u16')));
    }

    public static function readInt32(array &$reader): int
    {
        $value = self::readUInt32($reader);
        return $value >= 0x80000000 ? $value - 0x100000000 : $value;
    }

    public static function writeInt32(array &$writer, int $value): void
    {
        self::writeUInt32($writer, $value < 0 ? $value + 0x100000000 : $value);
    }

    public static function readUInt32(array &$reader): int
    {
        return unpack('Nvalue', self::readRawBytes($reader, 4))['value'];
    }

    public static function writeUInt32(array &$writer, int $value): void
    {
        self::appendPacked($writer, pack('N', self::intInRange($value, 0, 0xffffffff, 'u32')));
    }

    public static function readInt64(array &$reader): int
    {
        $parts = unpack('Nhi/Nlo', self::readRawBytes($reader, 8));
        $hi = $parts['hi'];
        $lo = $parts['lo'];
        if (($hi & 0x80000000) === 0) {
            return (int) ($hi * 0x100000000 + $lo);
        }
        if ($hi === 0x80000000 && $lo === 0) {
            return PHP_INT_MIN;
        }
        $hi = (~$hi) & 0xffffffff;
        $lo = (~$lo) & 0xffffffff;
        $magnitude = (int) ($hi * 0x100000000 + $lo + 1);
        return -$magnitude;
    }

    public static function writeInt64(array &$writer, int $value): void
    {
        if ($value >= 0) {
            self::writeUInt64($writer, $value);
            return;
        }
        if ($value === PHP_INT_MIN) {
            self::writeUInt32($writer, 0x80000000);
            self::writeUInt32($writer, 0);
            return;
        }
        $magnitude = -$value;
        $hi = intdiv($magnitude, 0x100000000);
        $lo = $magnitude % 0x100000000;
        $hi = (~$hi) & 0xffffffff;
        $lo = ((~$lo) & 0xffffffff) + 1;
        if ($lo > 0xffffffff) {
            $lo = 0;
            $hi = ($hi + 1) & 0xffffffff;
        }
        self::writeUInt32($writer, $hi);
        self::writeUInt32($writer, $lo);
    }

    public static function readUInt64(array &$reader): int
    {
        $parts = unpack('Nhi/Nlo', self::readRawBytes($reader, 8));
        $value = $parts['hi'] * 0x100000000 + $parts['lo'];
        if (!is_int($value) || $value > PHP_INT_MAX) {
            throw new \OverflowException('u64 value exceeds PHP integer range');
        }
        return $value;
    }

    public static function writeUInt64(array &$writer, int $value): void
    {
        if ($value < 0) {
            throw new \RangeException('u64 requires a non-negative integer');
        }
        $hi = intdiv($value, 0x100000000);
        $lo = $value % 0x100000000;
        self::writeUInt32($writer, $hi);
        self::writeUInt32($writer, $lo);
    }

    public static function readFloat32(array &$reader): float
    {
        return unpack('Gvalue', self::readRawBytes($reader, 4))['value'];
    }

    public static function writeFloat32(array &$writer, float $value): void
    {
        self::appendPacked($writer, pack('G', $value));
    }

    public static function readFloat64(array &$reader): float
    {
        return unpack('Evalue', self::readRawBytes($reader, 8))['value'];
    }

    public static function writeFloat64(array &$writer, float $value): void
    {
        self::appendPacked($writer, pack('E', $value));
    }

    public static function readString(array &$reader): string
    {
        $value = self::readByteString($reader);
        if (!preg_match('//u', $value)) {
            throw new \UnexpectedValueException('UniFFI string value was not valid UTF-8');
        }
        return $value;
    }

    public static function writeString(array &$writer, string $value): void
    {
        if (!preg_match('//u', $value)) {
            throw new \InvalidArgumentException('UniFFI string values must be valid UTF-8');
        }
        self::writeByteString($writer, $value);
    }

    public static function readByteString(array &$reader): string
    {
        $len = self::readInt32($reader);
        if ($len < 0) {
            throw new \UnexpectedValueException('Negative byte string length');
        }
        return self::readRawBytes($reader, $len);
    }

    public static function writeByteString(array &$writer, string $value): void
    {
        self::writeInt32($writer, strlen($value));
        self::appendPacked($writer, $value);
    }

    public static function readOptional(array &$reader, mixed $innerSpec): mixed
    {
        return match (self::readInt8($reader)) {
            0 => null,
            1 => self::readValue($reader, $innerSpec),
            default => throw new \UnexpectedValueException('Unexpected UniFFI optional tag'),
        };
    }

    public static function writeOptional(array &$writer, mixed $value, mixed $innerSpec): void
    {
        if ($value === null) {
            self::writeInt8($writer, 0);
            return;
        }
        self::writeInt8($writer, 1);
        self::writeValue($writer, $value, $innerSpec);
    }

    public static function readSequence(array &$reader, mixed $innerSpec): array
    {
        $len = self::readInt32($reader);
        if ($len < 0) {
            throw new \UnexpectedValueException('Negative sequence length');
        }
        $items = [];
        for ($i = 0; $i < $len; $i++) {
            $items[] = self::readValue($reader, $innerSpec);
        }
        return $items;
    }

    public static function writeSequence(array &$writer, array $value, mixed $innerSpec): void
    {
        self::writeInt32($writer, count($value));
        foreach ($value as $item) {
            self::writeValue($writer, $item, $innerSpec);
        }
    }

    public static function readMap(array &$reader, mixed $keySpec, mixed $valueSpec): array
    {
        $len = self::readInt32($reader);
        if ($len < 0) {
            throw new \UnexpectedValueException('Negative map length');
        }
        $map = [];
        for ($i = 0; $i < $len; $i++) {
            $key = self::readValue($reader, $keySpec);
            $map[$key] = self::readValue($reader, $valueSpec);
        }
        return $map;
    }

    public static function writeMap(array &$writer, array $value, mixed $keySpec, mixed $valueSpec): void
    {
        self::writeInt32($writer, count($value));
        foreach ($value as $key => $item) {
            self::writeValue($writer, $key, $keySpec);
            self::writeValue($writer, $item, $valueSpec);
        }
    }

    public static function readTimestamp(array &$reader): float
    {
        $seconds = self::readInt64($reader);
        $nanoseconds = self::readUInt32($reader);
        return $seconds >= 0
            ? $seconds + ($nanoseconds / 1.0e9)
            : $seconds - ($nanoseconds / 1.0e9);
    }

    public static function writeTimestamp(array &$writer, mixed $value): void
    {
        if ($value instanceof \DateTimeInterface) {
            $value = (float) $value->format('U.u');
        }
        $seconds = (int) $value;
        $nanoseconds = (int) abs(($value - $seconds) * 1.0e9);
        self::writeInt64($writer, $seconds);
        self::writeUInt32($writer, $nanoseconds);
    }

    public static function readDuration(array &$reader): float
    {
        $seconds = self::readUInt64($reader);
        $nanoseconds = self::readUInt32($reader);
        return $seconds + ($nanoseconds / 1.0e9);
    }

    public static function writeDuration(array &$writer, int|float $value): void
    {
        if ($value < 0) {
            throw new \RangeException('UniFFI duration cannot be negative');
        }
        $seconds = (int) $value;
        $nanoseconds = (int) (($value - $seconds) * 1.0e9);
        self::writeUInt64($writer, $seconds);
        self::writeUInt32($writer, $nanoseconds);
    }

    public static function insertCallbackObject(string $class, object $object): int
    {
        $handle = self::$nextCallbackHandle;
        self::$nextCallbackHandle += 2;
        self::$callbackHandles[$class][$handle] = $object;
        return $handle;
    }

    public static function cloneCallbackObject(string $class, int $handle): int
    {
        return self::insertCallbackObject($class, self::getCallbackObject($class, $handle));
    }

    public static function getCallbackObject(string $class, int $handle): object
    {
        if (!isset(self::$callbackHandles[$class][$handle])) {
            throw new \RuntimeException("UniFFI callback handle $handle for $class is stale");
        }
        return self::$callbackHandles[$class][$handle];
    }

    public static function removeCallbackObject(string $class, int $handle): void
    {
        unset(self::$callbackHandles[$class][$handle]);
    }

    public static function keepAlive(mixed $value): void
    {
        self::$keepAlive[] = $value;
    }

    public static function callbackSuccess(\FFI\CData $status): void
    {
        $status->code = self::CALL_SUCCESS;
        self::clearRustBuffer($status->errorBuf);
    }

    public static function callbackError(\FFI\CData $status, \Throwable $error): void
    {
        $status->code = self::CALL_UNEXPECTED_ERROR;
        self::copyRustBuffer($status->errorBuf, self::lowerString((string) $error));
    }

    private static function freeRustBuffer(\FFI\CData $buf): void
    {
        self::rustCall('{{ rustbuffer_free_fn }}', $buf);
    }

    private static function readRawBytes(array &$reader, int $len): string
    {
        if ($len < 0) {
            throw new \InvalidArgumentException('Cannot read a negative byte length');
        }
        $offset = $reader['offset'];
        $data = $reader['data'];
        if ($offset + $len > strlen($data)) {
            throw new \RuntimeException('Reading past the end of a UniFFI RustBuffer');
        }
        $reader['offset'] = $offset + $len;
        return substr($data, $offset, $len);
    }

    private static function appendPacked(array &$writer, string $bytes): void
    {
        $len = strlen($bytes);
        for ($i = 0; $i < $len; $i++) {
            $writer[] = ord($bytes[$i]);
        }
    }

    private static function bytesFromArray(array $writer): string
    {
        $bytes = '';
        foreach ($writer as $byte) {
            $bytes .= chr($byte & 0xff);
        }
        return $bytes;
    }

    private static function classForSpec(string $class): string
    {
        if (str_contains($class, '\\')) {
            return ltrim($class, '\\');
        }
        return __NAMESPACE__ . '\\' . $class;
    }

    private static function clearRustBuffer(\FFI\CData $buf): void
    {
        $buf->capacity = 0;
        $buf->len = 0;
        $buf->data = null;
    }

    private static function copyRustBuffer(\FFI\CData $target, \FFI\CData $source): void
    {
        $target->capacity = $source->capacity;
        $target->len = $source->len;
        $target->data = $source->data;
    }

    private static function intInRange(int $value, int $min, int $max, string $type): int
    {
        if ($value < $min || $value > $max) {
            throw new \RangeException("$type requires $min <= value <= $max");
        }
        return $value;
    }
}
