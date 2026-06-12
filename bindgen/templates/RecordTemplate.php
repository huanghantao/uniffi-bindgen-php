final class {{ class_name }}
{
    public function __construct({{ constructor_args }})
    {
    }

    public static function uniffiLift(\FFI\CData $buf): self
    {
        return UniFFIRuntime::liftSerialized($buf, ['record', self::class]);
    }

    public static function uniffiLower(self $value): \FFI\CData
    {
        return UniFFIRuntime::lowerSerialized($value, ['record', self::class]);
    }

    public static function uniffiRead(array &$reader): self
    {
        return new self({{ read_args }});
    }

    public static function uniffiWrite(self $value, array &$writer): void
    {
        {%- for field in write_fields %}
        UniFFIRuntime::writeValue($writer, $value->{{ field.property }}, {{ field.spec }});
        {%- endfor %}
    }
}

