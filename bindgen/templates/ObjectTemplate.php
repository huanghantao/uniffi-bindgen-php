{%- if obj.is_callback_trait %}
abstract class {{ obj.class_name }}
{
    public static function uniffiLift(int $handle): self
    {
        return {{ obj.proxy_class_name }}::uniffiAllocate($handle);
    }

    public static function uniffiLower(self $value): int
    {
        if ($value instanceof {{ obj.proxy_class_name }}) {
            return $value->uniffiCloneHandle();
        }
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

    {%- for meth in obj.methods %}
    abstract public function {{ meth.name }}({{ meth.args_decl }}){{ meth.return_type_decl }};
    {%- endfor %}
    {%- if obj.extension_code != "" %}

{{ obj.extension_code|safe }}
    {%- endif %}
}

final class {{ obj.proxy_class_name }} extends {{ obj.class_name }}
{
    private mixed $handle = 0;

    private function __construct()
    {
    }

    public function __destruct()
    {
        if ($this->handle !== 0) {
            try {
                UniFFIRuntime::rustCall('{{ obj.free_ffi_name }}', $this->handle);
            } catch (\Throwable) {
            }
            $this->handle = 0;
        }
    }

    public function uniffiCloneHandle(): int
    {
        if ($this->handle === 0) {
            throw new \RuntimeException('Cannot use a closed UniFFI object handle');
        }
        return UniFFIRuntime::rustCall('{{ obj.clone_ffi_name }}', $this->handle);
    }

    public static function uniffiAllocate(int $handle): self
    {
        $reflection = new \ReflectionClass(self::class);
        $instance = $reflection->newInstanceWithoutConstructor();
        $instance->handle = $handle;
        return $instance;
    }

    {%- for meth in obj.methods %}
    public function {{ meth.name }}({{ meth.args_decl }}){{ meth.return_type_decl }}
    {
        {%- if meth.has_return %}
        {%- if meth.throws_type_spec != "" %}
        $result = UniFFIRuntime::rustCallWithError('{{ meth.ffi_name }}', {{ meth.throws_type_spec }}, $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- else %}
        $result = UniFFIRuntime::rustCall('{{ meth.ffi_name }}', $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- endif %}
        return {{ meth.lift_expr }};
        {%- else %}
        {%- if meth.throws_type_spec != "" %}
        UniFFIRuntime::rustCallWithError('{{ meth.ffi_name }}', {{ meth.throws_type_spec }}, $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- else %}
        UniFFIRuntime::rustCall('{{ meth.ffi_name }}', $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- endif %}
        {%- endif %}
    }
    {%- endfor %}
    {%- if obj.extension_code != "" %}

{{ obj.extension_code|safe }}
    {%- endif %}
}
{%- else %}
final class {{ obj.class_name }}
{
    private mixed $handle = 0;

    {%- match obj.primary_constructor %}
    {%- when Some with (cons) %}
    public function __construct({{ cons.args_decl }})
    {
        {%- if cons.throws_type_spec != "" %}
        $this->handle = UniFFIRuntime::rustCallWithError('{{ cons.ffi_name }}', {{ cons.throws_type_spec }}{% if cons.args_call != "" %}, {{ cons.args_call }}{% endif %});
        {%- else %}
        $this->handle = UniFFIRuntime::rustCall('{{ cons.ffi_name }}'{% if cons.args_call != "" %}, {{ cons.args_call }}{% endif %});
        {%- endif %}
    }
    {%- when None %}
    private function __construct()
    {
    }
    {%- endmatch %}

    public function __destruct()
    {
        if ($this->handle !== 0) {
            try {
                UniFFIRuntime::rustCall('{{ obj.free_ffi_name }}', $this->handle);
            } catch (\Throwable) {
            }
            $this->handle = 0;
        }
    }

    public static function uniffiLift(int $handle): self
    {
        return self::uniffiAllocate($handle);
    }

    public static function uniffiLower(self $value): int
    {
        return $value->uniffiCloneHandle();
    }

    public static function uniffiRead(array &$reader): self
    {
        return self::uniffiLift(UniFFIRuntime::readUInt64($reader));
    }

    public static function uniffiWrite(self $value, array &$writer): void
    {
        UniFFIRuntime::writeUInt64($writer, self::uniffiLower($value));
    }

    public function uniffiCloneHandle(): int
    {
        if ($this->handle === 0) {
            throw new \RuntimeException('Cannot use a closed UniFFI object handle');
        }
        return UniFFIRuntime::rustCall('{{ obj.clone_ffi_name }}', $this->handle);
    }

    private static function uniffiAllocate(int $handle): self
    {
        $reflection = new \ReflectionClass(self::class);
        $instance = $reflection->newInstanceWithoutConstructor();
        $instance->handle = $handle;
        return $instance;
    }

    {%- for cons in obj.alternate_constructors %}
    public static function {{ cons.name }}({{ cons.args_decl }}): self
    {
        {%- if cons.throws_type_spec != "" %}
        $handle = UniFFIRuntime::rustCallWithError('{{ cons.ffi_name }}', {{ cons.throws_type_spec }}{% if cons.args_call != "" %}, {{ cons.args_call }}{% endif %});
        {%- else %}
        $handle = UniFFIRuntime::rustCall('{{ cons.ffi_name }}'{% if cons.args_call != "" %}, {{ cons.args_call }}{% endif %});
        {%- endif %}
        return self::uniffiAllocate($handle);
    }
    {%- endfor %}

    {%- for meth in obj.methods %}
    public function {{ meth.name }}({{ meth.args_decl }}){{ meth.return_type_decl }}
    {
        {%- if meth.has_return %}
        {%- if meth.throws_type_spec != "" %}
        $result = UniFFIRuntime::rustCallWithError('{{ meth.ffi_name }}', {{ meth.throws_type_spec }}, $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- else %}
        $result = UniFFIRuntime::rustCall('{{ meth.ffi_name }}', $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- endif %}
        return {{ meth.lift_expr }};
        {%- else %}
        {%- if meth.throws_type_spec != "" %}
        UniFFIRuntime::rustCallWithError('{{ meth.ffi_name }}', {{ meth.throws_type_spec }}, $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- else %}
        UniFFIRuntime::rustCall('{{ meth.ffi_name }}', $this->uniffiCloneHandle(){% if meth.args_call != "" %}, {{ meth.args_call }}{% endif %});
        {%- endif %}
        {%- endif %}
    }
    {%- endfor %}
    {%- if obj.extension_code != "" %}

{{ obj.extension_code|safe }}
    {%- endif %}
}
{%- endif %}
