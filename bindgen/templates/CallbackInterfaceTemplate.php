abstract class {{ class_name }}
{
    public static function uniffiLift(int $handle): self
    {
        throw new \RuntimeException('Cannot lift callback interface {{ class_name }} from Rust');
    }

    public static function uniffiLower(self $value): int
    {
        return UniFFIRuntime::insertCallbackObject(self::class, $value);
    }

    public static function uniffiRead(array &$reader): self
    {
        return self::uniffiLift(UniFFIRuntime::readUInt64($reader));
    }

    public static function uniffiWrite(self $value, array &$writer): void
    {
        UniFFIRuntime::writeUInt64($writer, self::uniffiLower($value));
    }

    {%- for method in methods %}
    abstract public function {{ method.name }}({{ method.args_decl }}){{ method.return_type_decl }};

    {%- endfor %}
}

