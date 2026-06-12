final class {{ helper_class }}
{
    private static bool $initialized = false;

    public static function init(): void
    {
        if (self::$initialized) {
            return;
        }

        $ffi = UniFFIRuntime::ffi();
        $vtable = $ffi->new('{{ vtable_name }}');
        $vtable->{'uniffi_free'} = function ($handle): void {
            UniFFIRuntime::removeCallbackObject({{ class_name }}::class, (int) $handle);
        };
        $vtable->{'uniffi_clone'} = function ($handle): int {
            return UniFFIRuntime::cloneCallbackObject({{ class_name }}::class, (int) $handle);
        };
        {%- for method in methods %}
        $vtable->{'{{ method.ffi_field_name }}'} = function ({{ method.closure_args }}): void {
            try {
                $obj = UniFFIRuntime::getCallbackObject({{ class_name }}::class, (int) $uniffiHandle);
{{ method.invoke_line }}{{ method.write_return }}                UniFFIRuntime::callbackSuccess($uniffiCallStatus);
            } catch (\Throwable $e) {
                UniFFIRuntime::callbackError($uniffiCallStatus, $e);
            }
        };
        {%- endfor %}
        $ffi->{'{{ init_fn }}'}(\FFI::addr($vtable));
        UniFFIRuntime::keepAlive($vtable);
        self::$initialized = true;
    }
}

