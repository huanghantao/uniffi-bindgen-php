final class {{ class_name }}
{
    private function __construct(
        public string $variant,
        public array $fields = [],
    ) {
    }

    {%- for variant in variants %}
    public static function {{ variant.method_name }}({{ variant.factory_args }}): self
    {
        return new self({{ variant.variant_literal }}, [{{ variant.field_array }}]);
    }

    {%- endfor %}
    public static function uniffiLift(\FFI\CData $buf): self
    {
        return UniFFIRuntime::liftSerialized($buf, ['enum', self::class]);
    }

    public static function uniffiLower(self $value): \FFI\CData
    {
        return UniFFIRuntime::lowerSerialized($value, ['enum', self::class]);
    }

    public static function uniffiRead(array &$reader): self
    {
        $variant = UniFFIRuntime::readInt32($reader);
        switch ($variant) {
            {%- for variant in variants %}
            case {{ variant.discr }}: return self::{{ variant.method_name }}({{ variant.read_args }});
            {%- endfor %}
            default:
                throw new \UnexpectedValueException('Unexpected enum discriminant for {{ class_name }}: ' . $variant);
        }
    }

    public static function uniffiWrite(self $value, array &$writer): void
    {
        switch ($value->variant) {
            {%- for variant in variants %}
            case {{ variant.variant_literal }}:
                UniFFIRuntime::writeInt32($writer, {{ variant.discr }});
                {%- for field in variant.write_fields %}
                UniFFIRuntime::writeValue($writer, $value->fields[{{ field.property_literal }}] ?? null, {{ field.spec }});
                {%- endfor %}
                return;
            {%- endfor %}
            default:
                throw new \UnexpectedValueException('Unexpected enum variant for {{ class_name }}: ' . $value->variant);
        }
    }
}
